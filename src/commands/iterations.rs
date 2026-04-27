use anyhow::Result;
use serde::Deserialize;
use tabled::{settings::Style, Table, Tabled};

use crate::client::AdoClient;

#[derive(Debug, Deserialize)]
struct IterationListResp {
    value: Vec<Iteration>,
}

#[derive(Debug, Deserialize)]
struct Iteration {
    name: String,
    path: String,
    #[serde(default)]
    attributes: IterationAttrs,
}

#[derive(Debug, Default, Deserialize)]
struct IterationAttrs {
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "finishDate")]
    finish_date: Option<String>,
    #[serde(rename = "timeFrame")]
    time_frame: Option<String>,
}

#[derive(Tabled)]
struct Row<'a> {
    #[tabled(rename = "TIMEFRAME")]
    timeframe: &'a str,
    #[tabled(rename = "NAME")]
    name: &'a str,
    #[tabled(rename = "PATH")]
    path: &'a str,
    #[tabled(rename = "DATES")]
    dates: String,
}

pub fn run(client: &AdoClient, json: bool) -> Result<()> {
    let v = client.get(&client.team_url("work/teamsettings/iterations"))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }
    let parsed: IterationListResp = serde_json::from_value(v)?;
    let rows: Vec<Row<'_>> = parsed
        .value
        .iter()
        .map(|it| Row {
            timeframe: it.attributes.time_frame.as_deref().unwrap_or("-"),
            name: &it.name,
            path: &it.path,
            dates: format_dates(&it.attributes),
        })
        .collect();
    println!("{}", Table::new(rows).with(Style::psql()));
    Ok(())
}

fn format_dates(a: &IterationAttrs) -> String {
    let s = a.start_date.as_deref().map(short_date).unwrap_or("-");
    let f = a.finish_date.as_deref().map(short_date).unwrap_or("-");
    format!("{s} → {f}")
}

fn short_date(d: &str) -> &str {
    // "2026-03-02T00:00:00Z" -> "2026-03-02"
    d.split('T').next().unwrap_or(d)
}
