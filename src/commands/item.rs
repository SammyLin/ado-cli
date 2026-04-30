use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

use crate::client::{urlencoding_minimal, AdoClient};

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

pub struct UpdateArgs {
    pub state: Option<String>,
    pub assignee: Option<String>,
    pub priority: Option<i64>,
    pub title: Option<String>,
    pub iteration: Option<String>,
    pub description: Option<String>,
    pub fields: Vec<String>,
    pub comment: Option<String>,
}

pub fn run_update(client: &AdoClient, id: u64, args: UpdateArgs, json_out: bool) -> Result<()> {
    let mut ops: Vec<Value> = Vec::new();
    if let Some(v) = &args.state {
        ops.push(json!({ "op": "add", "path": "/fields/System.State", "value": v }));
    }
    if let Some(v) = &args.assignee {
        ops.push(json!({ "op": "add", "path": "/fields/System.AssignedTo", "value": v }));
    }
    if let Some(v) = args.priority {
        ops.push(
            json!({ "op": "add", "path": "/fields/Microsoft.VSTS.Common.Priority", "value": v }),
        );
    }
    if let Some(v) = &args.title {
        ops.push(json!({ "op": "add", "path": "/fields/System.Title", "value": v }));
    }
    if let Some(v) = &args.iteration {
        ops.push(json!({ "op": "add", "path": "/fields/System.IterationPath", "value": v }));
    }
    if let Some(v) = &args.description {
        ops.push(json!({ "op": "add", "path": "/fields/System.Description", "value": v }));
    }
    for f in &args.fields {
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

    if let Some(text) = &args.comment {
        crate::commands::comment::run_add(client, id, text, json_out)?;
    }
    Ok(())
}

pub fn run_delete(client: &AdoClient, id: u64) -> Result<()> {
    let url = client.project_url(&format!("wit/workitems/{id}"));
    client.delete(&url)?;
    println!("deleted #{id} (moved to recycle bin)");
    Ok(())
}

pub fn run_list(
    client: &AdoClient,
    assignee: Option<String>,
    state: Option<String>,
    work_item_type: Option<String>,
    iteration: Option<String>,
    json_out: bool,
) -> Result<()> {
    let mut conditions = vec!["[System.TeamProject] = @project".to_string()];
    if let Some(a) = &assignee {
        conditions.push(format!("[System.AssignedTo] = '{}'", a.replace('\'', "''")));
    }
    if let Some(s) = &state {
        conditions.push(format!("[System.State] = '{}'", s.replace('\'', "''")));
    }
    if let Some(t) = &work_item_type {
        conditions.push(format!(
            "[System.WorkItemType] = '{}'",
            t.replace('\'', "''")
        ));
    }
    if let Some(it) = &iteration {
        conditions.push(format!(
            "[System.IterationPath] UNDER '{}'",
            it.replace('\'', "''")
        ));
    }
    let wiql = format!(
        "SELECT [System.Id] FROM WorkItems WHERE {} ORDER BY [System.Id] DESC",
        conditions.join(" AND ")
    );
    let url = client.project_url("wit/wiql");
    let body = json!({ "query": wiql });
    let v = client.post_json(&url, &body)?;
    let ids: Vec<u64> = v
        .get("workItems")
        .and_then(|w| w.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_u64()))
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        if json_out {
            println!("[]");
        } else {
            eprintln!("No work items found.");
        }
        return Ok(());
    }
    let batch_body = json!({
        "ids": ids,
        "fields": [
            "System.Id", "System.WorkItemType", "System.Title",
            "System.State", "System.AssignedTo",
            "Microsoft.VSTS.Common.Priority"
        ]
    });
    let batch_url = client.project_url("wit/workitemsbatch");
    let batch_resp = client.post_json(&batch_url, &batch_body)?;
    let items = batch_resp
        .get("value")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if json_out {
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        for w in &items {
            let id = w.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let f = w.get("fields").cloned().unwrap_or(Value::Null);
            let title = f
                .get("System.Title")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let state = f
                .get("System.State")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let wtype = f
                .get("System.WorkItemType")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let who = f
                .get("System.AssignedTo")
                .and_then(|a| a.get("displayName"))
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            println!("#{id}  [{wtype}]  {state}  {who}  {title}");
        }
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
            CreateSingleArgs {
                work_item_type: &args.work_item_type,
                title,
                parent_id: args.parent,
                description: args.description.as_deref(),
                assignee: args.assignee.as_deref(),
                priority: args.priority,
                iteration: iteration.as_deref(),
                area: area.as_deref(),
            },
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

struct CreateSingleArgs<'a> {
    work_item_type: &'a str,
    title: &'a str,
    parent_id: Option<u64>,
    description: Option<&'a str>,
    assignee: Option<&'a str>,
    priority: Option<i64>,
    iteration: Option<&'a str>,
    area: Option<&'a str>,
}

fn create_single(client: &AdoClient, args: CreateSingleArgs<'_>) -> Result<Value> {
    let mut ops: Vec<Value> = Vec::new();
    ops.push(json!({ "op": "add", "path": "/fields/System.Title", "value": args.title }));
    if let Some(d) = args.description {
        ops.push(json!({ "op": "add", "path": "/fields/System.Description", "value": d }));
    }
    if let Some(a) = args.assignee {
        ops.push(json!({ "op": "add", "path": "/fields/System.AssignedTo", "value": a }));
    }
    if let Some(p) = args.priority {
        ops.push(
            json!({ "op": "add", "path": "/fields/Microsoft.VSTS.Common.Priority", "value": p }),
        );
    }
    if let Some(it) = args.iteration {
        ops.push(json!({ "op": "add", "path": "/fields/System.IterationPath", "value": it }));
    }
    if let Some(ar) = args.area {
        ops.push(json!({ "op": "add", "path": "/fields/System.AreaPath", "value": ar }));
    }
    if let Some(pid) = args.parent_id {
        let parent_url = format!(
            "https://dev.azure.com/{}/_apis/wit/workItems/{}",
            client.org(),
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

    let url = client.project_url(&format!("wit/workitems/%24{}", args.work_item_type));
    client.post_patch(&url, &Value::Array(ops))
}

fn fetch_item_fields(client: &AdoClient, id: u64, fields: &[&str]) -> Result<Value> {
    let fields_param = urlencoding_minimal(&fields.join(","));
    let base = client.project_url(&format!("wit/workitems/{id}"));
    let url = format!("{base}&fields={fields_param}");
    let v = client.get(&url)?;
    Ok(v.get("fields").cloned().unwrap_or(Value::Null))
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
        println!("\n--- Description ---\n{}", super::strip_html(desc));
    }
    if let Some(ac) = f
        .get("Microsoft.VSTS.Common.AcceptanceCriteria")
        .and_then(|v| v.as_str())
    {
        println!("\n--- Acceptance Criteria ---\n{}", super::strip_html(ac));
    }
}

#[cfg(test)]
mod tests {}
