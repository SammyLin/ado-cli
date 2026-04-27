use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tabled::{settings::Style, Table, Tabled};

use crate::client::AdoClient;

#[derive(Debug, Deserialize)]
struct WiqlResp {
    #[serde(rename = "workItems")]
    work_items: Vec<WiqlRef>,
}

#[derive(Debug, Deserialize)]
struct WiqlRef {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct BatchResp {
    value: Vec<WorkItem>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkItem {
    id: u64,
    fields: Fields,
}

#[derive(Debug, Deserialize, Serialize)]
struct Fields {
    #[serde(rename = "System.WorkItemType")]
    work_item_type: String,
    #[serde(rename = "System.Title")]
    title: String,
    #[serde(rename = "System.State")]
    state: String,
    #[serde(rename = "System.AssignedTo")]
    assigned_to: Option<AssignedTo>,
    #[serde(rename = "Microsoft.VSTS.Scheduling.StoryPoints")]
    story_points: Option<f64>,
    #[serde(rename = "Microsoft.VSTS.Common.Priority")]
    priority: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AssignedTo {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Tabled)]
struct Row<'a> {
    #[tabled(rename = "ID")]
    id: u64,
    #[tabled(rename = "TYPE")]
    work_item_type: &'a str,
    #[tabled(rename = "STATE")]
    state: &'a str,
    #[tabled(rename = "SP")]
    story_points: String,
    #[tabled(rename = "PRI")]
    priority: String,
    #[tabled(rename = "ASSIGNEE")]
    assignee: String,
    #[tabled(rename = "TITLE")]
    title: &'a str,
}

pub fn run(client: &AdoClient, iteration: Option<String>, json: bool) -> Result<()> {
    let path = resolve_iteration_path(client, iteration.as_deref())?;
    let ids = wiql_ids_in_iteration(client, &path)?;
    if ids.is_empty() {
        if json {
            println!("[]");
        } else {
            eprintln!("No work items in iteration: {path}");
        }
        return Ok(());
    }
    let items = batch_get(client, &ids)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }
    let rows: Vec<Row<'_>> = items
        .iter()
        .map(|w| Row {
            id: w.id,
            work_item_type: &w.fields.work_item_type,
            state: &w.fields.state,
            story_points: w
                .fields
                .story_points
                .map(|n| format!("{n}"))
                .unwrap_or_else(|| "-".into()),
            priority: w
                .fields
                .priority
                .map(|n| format!("{n}"))
                .unwrap_or_else(|| "-".into()),
            assignee: w
                .fields
                .assigned_to
                .as_ref()
                .map(|a| a.display_name.clone())
                .unwrap_or_else(|| "-".into()),
            title: &w.fields.title,
        })
        .collect();
    eprintln!("Iteration: {path}  ({} items)", rows.len());
    println!("{}", Table::new(rows).with(Style::psql()));
    Ok(())
}

fn resolve_iteration_path(client: &AdoClient, iteration: Option<&str>) -> Result<String> {
    // Caller may pass a literal path or "current" (default).
    let want = iteration.unwrap_or("current");

    // Always fetch the iteration list — gives us both timeframe lookup and validation.
    let v = client.get(&client.team_url("work/teamsettings/iterations"))?;
    let arr = v
        .get("value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("iterations response missing 'value' array"))?;

    if want.eq_ignore_ascii_case("current") {
        for it in arr {
            let tf = it
                .get("attributes")
                .and_then(|a| a.get("timeFrame"))
                .and_then(|t| t.as_str());
            if tf == Some("current") {
                if let Some(p) = it.get("path").and_then(|p| p.as_str()) {
                    return Ok(p.to_string());
                }
            }
        }
        return Err(anyhow!("no iteration with timeFrame=current for this team"));
    }

    // Match either the full path or the bare iteration name.
    for it in arr {
        let path = it.get("path").and_then(|p| p.as_str()).unwrap_or("");
        let name = it.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if path == want || name == want {
            return Ok(path.to_string());
        }
    }
    Err(anyhow!(
        "iteration not found: {want} (use `ado iterations` to list valid paths)"
    ))
}

fn wiql_ids_in_iteration(client: &AdoClient, iteration_path: &str) -> Result<Vec<u64>> {
    let query = format!(
        "SELECT [System.Id] FROM WorkItems \
         WHERE [System.IterationPath] = '{}' \
         ORDER BY [System.WorkItemType], [System.Id]",
        iteration_path
    );
    let body = json!({ "query": query });
    let url = client.project_url("wit/wiql");
    let v = client.post_json(&url, &body)?;
    let parsed: WiqlResp = serde_json::from_value(v)?;
    Ok(parsed.work_items.into_iter().map(|w| w.id).collect())
}

fn batch_get(client: &AdoClient, ids: &[u64]) -> Result<Vec<WorkItem>> {
    let body = json!({
        "ids": ids,
        "fields": [
            "System.Id",
            "System.WorkItemType",
            "System.Title",
            "System.State",
            "System.AssignedTo",
            "System.Parent",
            "Microsoft.VSTS.Scheduling.StoryPoints",
            "Microsoft.VSTS.Common.Priority"
        ]
    });
    let url = client.project_url("wit/workitemsbatch");
    let v = client.post_json(&url, &body)?;
    let parsed: BatchResp = serde_json::from_value(v)?;
    Ok(parsed.value)
}
