use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Serialize;
use serde_json::Value;

use crate::config::Config;

const API_VERSION: &str = "7.1";

pub struct AdoClient {
    http: Client,
    cfg: Config,
    auth: String,
}

impl AdoClient {
    pub fn new(cfg: Config) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("build http client")?;
        let auth = format!("Basic {}", B64.encode(format!(":{}", cfg.pat)));
        Ok(Self { http, cfg, auth })
    }

    /// URL for project-scoped APIs: /<org>/<project>/_apis/...
    pub fn project_url(&self, path: &str) -> String {
        self.project_url_versioned(path, API_VERSION)
    }

    pub fn project_url_versioned(&self, path: &str, api_version: &str) -> String {
        format!(
            "https://dev.azure.com/{}/{}/_apis/{}?api-version={}",
            self.cfg.org, self.cfg.project, path, api_version
        )
    }

    /// URL for team-scoped APIs: /<org>/<project>/<team>/_apis/...
    pub fn team_url(&self, path: &str) -> String {
        let team = urlencoding_minimal(&self.cfg.team);
        format!(
            "https://dev.azure.com/{}/{}/{}/_apis/{}?api-version={}",
            self.cfg.org, self.cfg.project, team, path, API_VERSION
        )
    }

    pub fn get(&self, url: &str) -> Result<Value> {
        self.send(self.http.get(url))
    }

    pub fn post_json<T: Serialize>(&self, url: &str, body: &T) -> Result<Value> {
        self.send(
            self.http
                .post(url)
                .header(CONTENT_TYPE, "application/json")
                .json(body),
        )
    }

    /// POST with `application/json-patch+json` — used by ADO's "create work item" endpoint.
    pub fn post_patch(&self, url: &str, ops: &Value) -> Result<Value> {
        self.send(
            self.http
                .post(url)
                .header(CONTENT_TYPE, "application/json-patch+json")
                .json(ops),
        )
    }

    /// PATCH with `application/json-patch+json` — used by ADO's "update work item" endpoint.
    pub fn patch_json(&self, url: &str, ops: &Value) -> Result<Value> {
        self.send(
            self.http
                .patch(url)
                .header(CONTENT_TYPE, "application/json-patch+json")
                .json(ops),
        )
    }

    fn send(&self, req: RequestBuilder) -> Result<Value> {
        let resp = req
            .header(AUTHORIZATION, &self.auth)
            .header(ACCEPT, "application/json")
            .send()
            .context("http send")?;
        let status = resp.status();
        let body = resp.text().context("read response body")?;
        if !status.is_success() {
            // ADO returns an HTML page for 401/expired-PAT. Detect and shorten.
            let snippet = if body.trim_start().starts_with("<!DOCTYPE")
                || body.trim_start().starts_with("<html")
            {
                extract_html_title(&body).unwrap_or_else(|| "<html error page>".into())
            } else {
                body.chars().take(500).collect::<String>()
            };
            return Err(anyhow!("HTTP {}: {}", status, snippet));
        }
        serde_json::from_str(&body).with_context(|| format!("parse JSON: {body:.200}"))
    }
}

/// Minimal percent-encoder for path segments — enough to handle spaces in team names.
/// We stay conservative: only encode characters that ADO is sensitive to.
fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn extract_html_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title>")? + "<title>".len();
    let end = lower[start..].find("</title>")? + start;
    Some(html[start..end].trim().to_string())
}
