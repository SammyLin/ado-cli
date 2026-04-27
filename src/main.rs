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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::from_env()?;
    let client = AdoClient::new(cfg)?;

    match cli.cmd {
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
        },
    }
}
