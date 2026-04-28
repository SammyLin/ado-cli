mod client;
mod commands;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::client::AdoClient;
use crate::config::Config;

#[derive(Parser)]
#[command(
    name = "ado-cli",
    about = "Azure DevOps CLI — sprint cards, iterations, work items",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Initialize a .ado.toml config file in the current directory.
    Init,
    /// List iterations (sprints) for the configured team.
    Iterations {
        #[arg(long)]
        json: bool,
    },
    /// Sprint operations.
    Sprint {
        #[command(subcommand)]
        cmd: SprintCmd,
    },
    /// Work item operations: show / create / comment.
    Item {
        #[command(subcommand)]
        cmd: ItemCmd,
    },
}

#[derive(Subcommand)]
enum SprintCmd {
    /// List work items in an iteration (defaults to the current sprint).
    List {
        /// Iteration name or full path. Defaults to "current".
        #[arg(long)]
        iteration: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ItemCmd {
    /// Show a single work item with description + acceptance criteria.
    Show {
        id: u64,
        #[arg(long)]
        json: bool,
    },
    /// Create one or more work items. Pass `--title` multiple times for batch.
    Create {
        /// Title for the work item. Repeat for batch creation.
        #[arg(long, required = true)]
        title: Vec<String>,
        /// Work item type. Examples: Task, Bug, "User Story", Epic.
        #[arg(long, default_value = "Task")]
        r#type: String,
        /// Parent work item id. Inherits the parent's iteration + area unless
        /// `--iteration` / `--area` are passed explicitly.
        #[arg(long)]
        parent: Option<u64>,
        /// Optional description (HTML allowed). Applied to every created item.
        #[arg(long)]
        description: Option<String>,
        /// Assignee (user email or unique name).
        #[arg(long)]
        assignee: Option<String>,
        /// Priority (1=highest, 4=lowest).
        #[arg(long)]
        priority: Option<i64>,
        /// Iteration path. Overrides the parent's iteration.
        #[arg(long)]
        iteration: Option<String>,
        /// Area path. Overrides the parent's area.
        #[arg(long)]
        area: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Update fields on an existing work item (state transitions, reassign, etc.).
    Update {
        id: u64,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        priority: Option<i64>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        iteration: Option<String>,
        #[arg(long)]
        description: Option<String>,
        /// Repeatable: --field 'Custom.MyField=value'
        #[arg(long = "field")]
        fields: Vec<String>,
        /// Also append a comment after the update succeeds.
        #[arg(long)]
        comment: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Delete a work item (moves to recycle bin).
    Delete {
        id: u64,
    },
    /// List work items matching filters (uses WIQL).
    List {
        /// Filter by assignee (uniqueName, e.g. user@example.com).
        #[arg(long)]
        assignee: Option<String>,
        /// Filter by state (e.g. New, Active, Done, Closed).
        #[arg(long)]
        state: Option<String>,
        /// Filter by work item type (e.g. Task, Bug, "User Story").
        #[arg(long)]
        r#type: Option<String>,
        /// Filter by iteration path.
        #[arg(long)]
        iteration: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Comment operations on a work item.
    Comment {
        #[command(subcommand)]
        cmd: CommentCmd,
    },
    /// Link operations on a work item.
    Link {
        #[command(subcommand)]
        cmd: LinkCmd,
    },
}

#[derive(Subcommand)]
enum CommentCmd {
    /// List comments on a work item.
    List {
        id: u64,
        #[arg(long)]
        json: bool,
    },
    /// Add a comment to a work item.
    Add {
        id: u64,
        #[arg(long, required = true)]
        text: String,
        #[arg(long)]
        json: bool,
    },
    /// Update an existing comment.
    Update {
        id: u64,
        /// Comment ID to update.
        #[arg(long)]
        comment_id: u64,
        #[arg(long, required = true)]
        text: String,
        #[arg(long)]
        json: bool,
    },
    /// Delete a comment.
    Delete {
        id: u64,
        /// Comment ID to delete.
        #[arg(long)]
        comment_id: u64,
    },
}

#[derive(Subcommand)]
enum LinkCmd {
    /// List links (relations) on a work item.
    List {
        id: u64,
        #[arg(long)]
        json: bool,
    },
    /// Add a link between two work items.
    Add {
        id: u64,
        /// Target work item ID to link to.
        #[arg(long)]
        target: u64,
        /// Link type: parent, child, related, duplicate, duplicate-of, predecessor, successor.
        #[arg(long = "type")]
        link_type: String,
        /// Optional comment on the link.
        #[arg(long)]
        comment: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Link a commit to a work item.
    AddCommit {
        id: u64,
        /// Repository name in Azure DevOps.
        #[arg(long)]
        repo: String,
        /// Commit SHA (full or prefix).
        #[arg(long)]
        commit: String,
        /// Optional comment on the link.
        #[arg(long)]
        comment: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Remove a link between two work items.
    Remove {
        id: u64,
        /// Target work item ID to unlink.
        #[arg(long)]
        target: u64,
        /// Link type to remove.
        #[arg(long = "type")]
        link_type: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Init doesn't need config.
    if matches!(cli.cmd, Cmd::Init) {
        return run_init();
    }

    let cfg = Config::load()?;
    let client = AdoClient::new(cfg)?;

    match cli.cmd {
        Cmd::Init => unreachable!(),
        Cmd::Iterations { json } => commands::iterations::run(&client, json),
        Cmd::Sprint { cmd } => match cmd {
            SprintCmd::List { iteration, json } => commands::sprint::run(&client, iteration, json),
        },
        Cmd::Item { cmd } => match cmd {
            ItemCmd::Show { id, json } => commands::item::run_show(&client, id, json),
            ItemCmd::Create {
                title,
                r#type,
                parent,
                description,
                assignee,
                priority,
                iteration,
                area,
                json,
            } => commands::item::run_create(
                &client,
                commands::item::CreateArgs {
                    work_item_type: r#type,
                    titles: title,
                    parent,
                    description,
                    assignee,
                    priority,
                    iteration,
                    area,
                },
                json,
            ),
            ItemCmd::Comment { cmd } => match cmd {
                CommentCmd::List { id, json } => commands::comment::run_list(&client, id, json),
                CommentCmd::Add { id, text, json } => {
                    commands::comment::run_add(&client, id, &text, json)
                }
                CommentCmd::Update { id, comment_id, text, json } => {
                    commands::comment::run_update(&client, id, comment_id, &text, json)
                }
                CommentCmd::Delete { id, comment_id } => {
                    commands::comment::run_delete(&client, id, comment_id)
                }
            },
            ItemCmd::Update {
                id,
                state,
                assignee,
                priority,
                title,
                iteration,
                description,
                fields,
                comment,
                json,
            } => commands::item::run_update(
                &client,
                id,
                commands::item::UpdateArgs {
                    state,
                    assignee,
                    priority,
                    title,
                    iteration,
                    description,
                    fields,
                    comment,
                },
                json,
            ),
            ItemCmd::Delete { id } => commands::item::run_delete(&client, id),
            ItemCmd::List { assignee, state, r#type, iteration, json } => {
                commands::item::run_list(&client, assignee, state, r#type, iteration, json)
            }
            ItemCmd::Link { cmd } => match cmd {
                LinkCmd::List { id, json } => commands::link::run_list(&client, id, json),
                LinkCmd::Add { id, target, link_type, comment, json } => {
                    commands::link::run_add(&client, id, target, &link_type, comment.as_deref(), json)
                }
                LinkCmd::AddCommit { id, repo, commit, comment, json } => {
                    commands::link::run_add_commit(&client, id, &repo, &commit, comment.as_deref(), json)
                }
                LinkCmd::Remove { id, target, link_type } => {
                    commands::link::run_remove(&client, id, target, &link_type)
                }
            },
        },
    }
}

fn run_init() -> Result<()> {
    use crate::config::CONFIG_FILE;
    use std::io::{self, BufRead, Write};
    use std::path::Path;

    let path = Path::new(CONFIG_FILE);
    if path.exists() {
        eprintln!("{CONFIG_FILE} already exists. Overwrite? [y/N] ");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            eprintln!("aborted.");
            return Ok(());
        }
    }

    let org = prompt("Organization (ADO_ORG)")?;
    let project = prompt("Project (ADO_PROJECT)")?;
    let team = prompt("Team (ADO_TEAM)")?;
    let pat = prompt("Personal Access Token (ADO_PAT)")?;

    let content = format!(
        "org = \"{org}\"\nproject = \"{project}\"\nteam = \"{team}\"\npat = \"{pat}\"\n"
    );
    std::fs::write(path, &content)?;
    println!("wrote {CONFIG_FILE}");
    println!("hint: add {CONFIG_FILE} to .gitignore (it contains your PAT)");
    Ok(())
}

fn prompt(label: &str) -> Result<String> {
    use std::io::{self, BufRead, Write};
    eprint!("{label}: ");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let val = line.trim().to_string();
    if val.is_empty() {
        return Err(anyhow::anyhow!("{label} cannot be empty"));
    }
    Ok(val)
}
