use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

use crate::client::AdoClient;

pub fn run_show(client: &AdoClient, id: u64, json_out: bool) -> Result<()> {
    let url = client.project_url(&format!("wit/workitems/{id}"));
    // $expand=all returns description, AC, relations, history.
    let url_expanded = format!("{url}&$expand=all");
    let v = client.get(&url_expanded)?;

    if json_out {
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }
    print_human(&v);
    Ok(())
}

pub struct CreateArgs {
    pub work_item_type: String,
    pub titles: Vec<String>,
    pub parent: Option<u64>,
    pub description: Option<String>,
    pub assignee: Option<String>,
    pub priority: Option<i64>,
    pub iteration: Option<String>,
    pub area: Option<String>,
}

pub fn run_update(
    client: &AdoClient,
    id: u64,
    state: Option<String>,
    assignee: Option<String>,
    priority: Option<i64>,
    title: Option<String>,
    iteration: Option<String>,
    description: Option<String>,
    fields: Vec<String>,
    comment: Option<String>,
    json_out: bool,
) -> Result<()> {
    let mut ops: Vec<Value> = Vec::new();
    if let Some(v) = &state {
        ops.push(json!({ "op": "add", "path": "/fields/System.State", "value": v }));
    }
    if let Some(v) = &assignee {
        ops.push(json!({ "op": "add", "path": "/fields/System.AssignedTo", "value": v }));
    }
    if let Some(v) = priority {
        ops.push(json!({ "op": "add", "path": "/fields/Microsoft.VSTS.Common.Priority", "value": v }));
    }
    if let Some(v) = &title {
        ops.push(json!({ "op": "add", "path": "/fields/System.Title", "value": v }));
    }
    if let Some(v) = &iteration {
        ops.push(json!({ "op": "add", "path": "/fields/System.IterationPath", "value": v }));
    }
    if let Some(v) = &description {
        ops.push(json!({ "op": "add", "path": "/fields/System.Description", "value": v }));
    }
    for f in &fields {
        let (key, val) = f
            .split_once('=')
            .ok_or_else(|| anyhow!("--field must be key=value, got: {f}"))?;
        ops.push(json!({ "op": "add", "path": format!("/fields/{key}"), "value": val }));
    }
    if ops.is_empty() {
        return Err(anyhow!("at least one field flag is required"));
    }

    let url = client.project_url(&format!("wit/workitems/{id}"));
    let v = client.patch_json(&url, &Value::Array(ops))?;

    if json_out {
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        let new_state = v
            .get("fields")
            .and_then(|f| f.get("System.State"))
            .and_then(|s| s.as_str())
            .unwrap_or("-");
        println!("updated #{id}  State={new_state}");
    }

    if let Some(text) = &comment {
        crate::commands::comment::run_add(client, id, text, json_out)?;
    }
    Ok(())
}

pub fn run_create(client: &AdoClient, args: CreateArgs, json_out: bool) -> Result<()> {
    if args.titles.is_empty() {
        return Err(anyhow!("at least one --title is required"));
    }

    // If parent is given and iteration/area not specified, inherit from parent.
    let (parent_iteration, parent_area) = if let Some(pid) = args.parent {
        let parent = fetch_item_fields(client, pid, &["System.IterationPath", "System.AreaPath"])
            .with_context(|| format!("fetch parent #{pid}"))?;
        (
            parent
                .get("System.IterationPath")
                .and_then(|v| v.as_str())
                .map(String::from),
            parent
                .get("System.AreaPath")
                .and_then(|v| v.as_str())
                .map(String::from),
        )
    } else {
        (None, None)
    };

    let iteration = args.iteration.clone().or(parent_iteration);
    let area = args.area.clone().or(parent_area);

    let mut created = Vec::with_capacity(args.titles.len());
    for title in &args.titles {
        let v = create_single(
            client,
            &args.work_item_type,
            title,
            args.parent,
            args.description.as_deref(),
            args.assignee.as_deref(),
            args.priority,
            iteration.as_deref(),
            area.as_deref(),
        )
        .with_context(|| format!("create work item: {title}"))?;
        created.push(v);
    }

    if json_out {
        println!("{}", serde_json::to_string_pretty(&created)?);
    } else {
        for v in &created {
            let id = v.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let title = v
                .get("fields")
                .and_then(|f| f.get("System.Title"))
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            println!("created #{id}  {title}");
        }
    }
    Ok(())
}

fn create_single(
    client: &AdoClient,
    work_item_type: &str,
    title: &str,
    parent_id: Option<u64>,
    description: Option<&str>,
    assignee: Option<&str>,
    priority: Option<i64>,
    iteration: Option<&str>,
    area: Option<&str>,
) -> Result<Value> {
    let mut ops: Vec<Value> = Vec::new();
    ops.push(json!({ "op": "add", "path": "/fields/System.Title", "value": title }));
    if let Some(d) = description {
        ops.push(json!({ "op": "add", "path": "/fields/System.Description", "value": d }));
    }
    if let Some(a) = assignee {
        ops.push(json!({ "op": "add", "path": "/fields/System.AssignedTo", "value": a }));
    }
    if let Some(p) = priority {
        ops.push(
            json!({ "op": "add", "path": "/fields/Microsoft.VSTS.Common.Priority", "value": p }),
        );
    }
    if let Some(it) = iteration {
        ops.push(json!({ "op": "add", "path": "/fields/System.IterationPath", "value": it }));
    }
    if let Some(ar) = area {
        ops.push(json!({ "op": "add", "path": "/fields/System.AreaPath", "value": ar }));
    }
    if let Some(pid) = parent_id {
        let parent_url = format!(
            "https://dev.azure.com/{}/_apis/wit/workItems/{}",
            client_org(client),
            pid
        );
        ops.push(json!({
            "op": "add",
            "path": "/relations/-",
            "value": {
                "rel": "System.LinkTypes.Hierarchy-Reverse",
                "url": parent_url
            }
        }));
    }

    // ADO's "$Type" prefix is literal — percent-encoded for safety.
    let url = client.project_url(&format!("wit/workitems/%24{work_item_type}"));
    client.post_patch(&url, &Value::Array(ops))
}

fn fetch_item_fields(client: &AdoClient, id: u64, fields: &[&str]) -> Result<Value> {
    let fields_param = urlencoding_path(&fields.join(","));
    let base = client.project_url(&format!("wit/workitems/{id}"));
    let url = format!("{base}&fields={fields_param}");
    let v = client.get(&url)?;
    Ok(v.get("fields").cloned().unwrap_or(Value::Null))
}

fn client_org(client: &AdoClient) -> String {
    // Extract org from any project URL — cheaper than re-exposing config.
    let probe = client.project_url("");
    // probe = "https://dev.azure.com/<org>/<project>/_apis/?api-version=7.1"
    probe
        .strip_prefix("https://dev.azure.com/")
        .and_then(|s| s.split('/').next())
        .unwrap_or("")
        .to_string()
}

fn urlencoding_path(s: &str) -> String {
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

fn print_human(v: &Value) {
    let f = v.get("fields").cloned().unwrap_or(Value::Null);
    let id = v.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
    let get = |k: &str| f.get(k).and_then(|v| v.as_str()).unwrap_or("-").to_string();
    let assignee = f
        .get("System.AssignedTo")
        .and_then(|a| a.get("displayName"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let sp = f
        .get("Microsoft.VSTS.Scheduling.StoryPoints")
        .and_then(|v| v.as_f64())
        .map(|n| format!("{n}"))
        .unwrap_or_else(|| "-".into());
    let pri = f
        .get("Microsoft.VSTS.Common.Priority")
        .and_then(|v| v.as_i64())
        .map(|n| format!("{n}"))
        .unwrap_or_else(|| "-".into());

    println!(
        "#{id}  [{}]  {}",
        get("System.WorkItemType"),
        get("System.Title")
    );
    println!("State:    {}", get("System.State"));
    println!("Iteration:{}", get("System.IterationPath"));
    println!("Area:     {}", get("System.AreaPath"));
    println!("Assignee: {assignee}");
    println!("SP={sp}  Priority={pri}");

    if let Some(desc) = f.get("System.Description").and_then(|v| v.as_str()) {
        println!("\n--- Description ---\n{}", strip_html(desc));
    }
    if let Some(ac) = f
        .get("Microsoft.VSTS.Common.AcceptanceCriteria")
        .and_then(|v| v.as_str())
    {
        println!("\n--- Acceptance Criteria ---\n{}", strip_html(ac));
    }
}

/// Crude HTML → text: drop tags, decode common entities. ADO descriptions are short
/// and a real HTML parser is overkill for terminal preview.
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("<p>hello <b>world</b></p>"), "hello world");
    }

    #[test]
    fn strip_html_decodes_entities() {
        assert_eq!(strip_html("a &amp; b &lt; c &gt; d"), "a & b < c > d");
    }

    #[test]
    fn strip_html_nbsp() {
        assert_eq!(strip_html("foo&nbsp;bar"), "foo bar");
    }

    #[test]
    fn strip_html_quot() {
        assert_eq!(strip_html("say &quot;hi&quot;"), "say \"hi\"");
    }

    #[test]
    fn strip_html_plain_text_passthrough() {
        assert_eq!(strip_html("no tags here"), "no tags here");
    }

    #[test]
    fn urlencoding_path_preserves_comma() {
        assert_eq!(urlencoding_path("a,b,c"), "a,b,c");
    }

    #[test]
    fn urlencoding_path_encodes_spaces() {
        assert_eq!(urlencoding_path("a b"), "a%20b");
    }
}
