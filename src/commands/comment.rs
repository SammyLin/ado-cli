use anyhow::Result;
use serde_json::json;

use crate::client::AdoClient;

const COMMENTS_API_VERSION: &str = "7.1-preview.4";

pub fn run_list(client: &AdoClient, id: u64, json_out: bool) -> Result<()> {
    let url = client.project_url_versioned(
        &format!("wit/workItems/{id}/comments"),
        COMMENTS_API_VERSION,
    );
    let v = client.get(&url)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }
    let comments = v
        .get("comments")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();
    if comments.is_empty() {
        eprintln!("(no comments on #{id})");
        return Ok(());
    }
    for c in comments {
        let cid = c.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let when = c.get("createdDate").and_then(|v| v.as_str()).unwrap_or("-");
        let who = c
            .get("createdBy")
            .and_then(|u| u.get("displayName"))
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let text = c.get("text").and_then(|v| v.as_str()).unwrap_or("");
        println!("--- comment {cid}  by {who}  at {when} ---");
        println!("{}", super::strip_html(text));
        println!();
    }
    Ok(())
}

pub fn run_add(client: &AdoClient, id: u64, text: &str, json_out: bool) -> Result<()> {
    let url = client.project_url_versioned(
        &format!("wit/workItems/{id}/comments"),
        COMMENTS_API_VERSION,
    );
    let body = json!({ "text": text });
    let v = client.post_json(&url, &body)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        let cid = v.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("added comment {cid} on #{id}");
    }
    Ok(())
}


