use anyhow::{anyhow, Result};
use serde_json::json;

use crate::client::AdoClient;

/// Common work-item link relation types (forward direction).
const LINK_TYPES: &[(&str, &str)] = &[
    ("parent", "System.LinkTypes.Hierarchy-Reverse"),
    ("child", "System.LinkTypes.Hierarchy-Forward"),
    ("related", "System.LinkTypes.Related"),
    ("duplicate", "System.LinkTypes.Duplicate-Forward"),
    ("duplicate-of", "System.LinkTypes.Duplicate-Reverse"),
    ("predecessor", "System.LinkTypes.Dependency-Reverse"),
    ("successor", "System.LinkTypes.Dependency-Forward"),
];

fn resolve_relation(name: &str) -> Result<&'static str> {
    // Accept either a friendly alias or a raw System.LinkTypes.* reference name.
    for &(alias, ref_name) in LINK_TYPES {
        if name.eq_ignore_ascii_case(alias) {
            return Ok(ref_name);
        }
    }
    if name.starts_with("System.LinkTypes.") || name == "Hyperlink" {
        // Caller passed a raw ref name — trust it.
        // We can't return a &'static str for arbitrary input, so we match known ones.
        for &(_, ref_name) in LINK_TYPES {
            if name == ref_name {
                return Ok(ref_name);
            }
        }
    }
    let aliases: Vec<&str> = LINK_TYPES.iter().map(|(a, _)| *a).collect();
    Err(anyhow!(
        "unknown link type: {name}\nvalid aliases: {}",
        aliases.join(", ")
    ))
}

fn work_item_url(client: &AdoClient, id: u64) -> String {
    format!(
        "https://dev.azure.com/{}/_apis/wit/workItems/{}",
        client.org(),
        id
    )
}

pub fn run_add(
    client: &AdoClient,
    id: u64,
    target: u64,
    link_type: &str,
    comment: Option<&str>,
    json_out: bool,
) -> Result<()> {
    let rel = resolve_relation(link_type)?;
    let mut link_value = json!({
        "rel": rel,
        "url": work_item_url(client, target),
    });
    if let Some(c) = comment {
        link_value["attributes"] = json!({ "comment": c });
    }
    let ops = json!([{ "op": "add", "path": "/relations/-", "value": link_value }]);
    let url = client.project_url(&format!("wit/workitems/{id}"));
    let v = client.patch_json(&url, &ops)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        println!("added {link_type} link: #{id} → #{target}");
    }
    Ok(())
}

pub fn run_list(client: &AdoClient, id: u64, json_out: bool) -> Result<()> {
    let base = client.project_url(&format!("wit/workitems/{id}"));
    let url = format!("{base}&$expand=relations");
    let v = client.get(&url)?;

    let relations = v
        .get("relations")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    if json_out {
        println!("{}", serde_json::to_string_pretty(&relations)?);
        return Ok(());
    }
    if relations.is_empty() {
        eprintln!("(no links on #{id})");
        return Ok(());
    }
    for r in &relations {
        let rel = r.get("rel").and_then(|v| v.as_str()).unwrap_or("-");
        let url_str = r.get("url").and_then(|v| v.as_str()).unwrap_or("-");
        let comment = r
            .get("attributes")
            .and_then(|a| a.get("comment"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // Try to extract target work item ID from URL.
        let target = url_str
            .rsplit('/')
            .next()
            .and_then(|s| s.parse::<u64>().ok());
        let friendly = LINK_TYPES
            .iter()
            .find(|(_, rn)| *rn == rel)
            .map(|(a, _)| *a)
            .unwrap_or(rel);
        if let Some(tid) = target {
            if comment.is_empty() {
                println!("{friendly}  → #{tid}");
            } else {
                println!("{friendly}  → #{tid}  ({comment})");
            }
        } else {
            // Non-work-item link (hyperlink, artifact, etc.)
            if comment.is_empty() {
                println!("{friendly}  → {url_str}");
            } else {
                println!("{friendly}  → {url_str}  ({comment})");
            }
        }
    }
    Ok(())
}

pub fn run_remove(client: &AdoClient, id: u64, target: u64, link_type: &str) -> Result<()> {
    let rel = resolve_relation(link_type)?;
    let target_url = work_item_url(client, target);

    // Fetch current relations to find the index.
    let base = client.project_url(&format!("wit/workitems/{id}"));
    let url = format!("{base}&$expand=relations");
    let v = client.get(&url)?;
    let relations = v
        .get("relations")
        .and_then(|r| r.as_array())
        .ok_or_else(|| anyhow!("work item #{id} has no relations"))?;

    let idx = relations
        .iter()
        .position(|r| {
            r.get("rel").and_then(|v| v.as_str()) == Some(rel)
                && r.get("url").and_then(|v| v.as_str()) == Some(&target_url)
        })
        .ok_or_else(|| anyhow!("no {link_type} link to #{target} found on #{id}"))?;

    let ops = json!([{ "op": "remove", "path": format!("/relations/{idx}") }]);
    client.patch_json(&base, &ops)?;
    println!("removed {link_type} link: #{id} → #{target}");
    Ok(())
}

pub fn run_add_commit(
    client: &AdoClient,
    id: u64,
    repo: &str,
    commit: &str,
    comment: Option<&str>,
    json_out: bool,
) -> Result<()> {
    // Look up repo to get projectId and repoId.
    let repo_url = client.project_url(&format!("git/repositories/{repo}"));
    let repo_info = client.get(&repo_url)?;
    let repo_id = repo_info
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("could not resolve repo '{repo}'"))?;
    let project_id = repo_info
        .get("project")
        .and_then(|p| p.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("could not resolve projectId for repo '{repo}'"))?;

    let artifact_url = format!("vstfs:///Git/Commit/{project_id}/{repo_id}/{commit}");
    let mut link_value = json!({
        "rel": "ArtifactLink",
        "url": artifact_url,
        "attributes": {
            "name": "Fixed in Commit"
        }
    });
    if let Some(c) = comment {
        link_value["attributes"]["comment"] = json!(c);
    }
    let ops = json!([{ "op": "add", "path": "/relations/-", "value": link_value }]);
    let url = client.project_url(&format!("wit/workitems/{id}"));
    let v = client.patch_json(&url, &ops)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        println!("added commit link: #{id} → {repo}/{}", &commit[..commit.len().min(8)]);
    }
    Ok(())
}
