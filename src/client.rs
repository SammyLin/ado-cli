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

    /// PATCH with `application/json` — used by ADO's "update comment" endpoint.
    pub fn patch_json_body<T: Serialize>(&self, url: &str, body: &T) -> Result<Value> {
        self.send(
            self.http
                .patch(url)
                .header(CONTENT_TYPE, "application/json")
                .json(body),
        )
    }

    pub fn delete(&self, url: &str) -> Result<()> {
        let resp = self
            .http
            .delete(url)
            .header(AUTHORIZATION, &self.auth)
            .header(ACCEPT, "application/json")
            .send()
            .context("http send")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().context("read response body")?;
            let snippet = body.chars().take(500).collect::<String>();
            return Err(anyhow!("HTTP {}: {}", status, snippet));
        }
        Ok(())
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

    pub fn org(&self) -> &str {
        &self.cfg.org
    }

    pub fn repo(&self) -> Option<&str> {
        self.cfg.repo.as_deref()
    }
}

/// Minimal percent-encoder for path segments — enough to handle spaces in team names.
/// We stay conservative: only encode characters that ADO is sensitive to.
pub fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b',' => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoding_minimal_plain() {
        assert_eq!(urlencoding_minimal("hello"), "hello");
    }

    #[test]
    fn urlencoding_minimal_spaces() {
        assert_eq!(urlencoding_minimal("My Team"), "My%20Team");
    }

    #[test]
    fn urlencoding_minimal_special_chars() {
        assert_eq!(urlencoding_minimal("a/b&c"), "a%2Fb%26c");
    }

    #[test]
    fn urlencoding_minimal_preserves_unreserved() {
        assert_eq!(urlencoding_minimal("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn urlencoding_minimal_preserves_comma() {
        assert_eq!(urlencoding_minimal("a,b,c"), "a,b,c");
    }

    #[test]
    fn extract_html_title_basic() {
        let html = "<html><head><title>Access Denied</title></head></html>";
        assert_eq!(extract_html_title(html), Some("Access Denied".into()));
    }

    #[test]
    fn extract_html_title_case_insensitive() {
        let html = "<HTML><HEAD><TITLE>Error Page</TITLE></HEAD></HTML>";
        assert_eq!(extract_html_title(html), Some("Error Page".into()));
    }

    #[test]
    fn extract_html_title_none_when_missing() {
        assert_eq!(
            extract_html_title("<html><body>no title</body></html>"),
            None
        );
    }

    #[test]
    fn extract_html_title_trims_whitespace() {
        let html = "<title>  Spaced  </title>";
        assert_eq!(extract_html_title(html), Some("Spaced".into()));
    }

    #[test]
    fn project_url_format() {
        let cfg = Config {
            org: "MyOrg".into(),
            project: "MyProj".into(),
            team: "T".into(),
            pat: "fake".into(),
            repo: None,
        };
        let client = AdoClient::new(cfg).unwrap();
        assert_eq!(
            client.project_url("wit/workitems/1"),
            "https://dev.azure.com/MyOrg/MyProj/_apis/wit/workitems/1?api-version=7.1"
        );
    }

    #[test]
    fn team_url_encodes_spaces() {
        let cfg = Config {
            org: "O".into(),
            project: "P".into(),
            team: "My Team".into(),
            pat: "x".into(),
            repo: None,
        };
        let client = AdoClient::new(cfg).unwrap();
        let url = client.team_url("work/teamsettings/iterations");
        assert!(url.contains("/My%20Team/"));
        assert!(url.ends_with("?api-version=7.1"));
    }
}
