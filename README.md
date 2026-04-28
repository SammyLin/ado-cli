# ado-cli

[![Tests](https://github.com/SammyLin/ado-cli/actions/workflows/test.yml/badge.svg)](https://github.com/SammyLin/ado-cli/actions/workflows/test.yml)
[![Rust](https://img.shields.io/badge/Rust-2021-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A tiny Rust CLI for querying and mutating Azure DevOps work items — sprint cards, iterations, single items, and comments.

## Features

- **Sprint Board**: List work items in the current or any named sprint
- **Iterations**: View all iterations with timeframe and date ranges
- **Work Items**: Show, create, update, delete, and list with filters
- **Comments**: List, add, update, and delete comments on any work item
- **Parent Inheritance**: New items inherit iteration + area from `--parent` automatically
- **JSON Output**: Every read/write command supports `--json` for machine-readable output

## Installation

### Homebrew (recommended)

```bash
brew tap SammyLin/tap
brew install ado-cli
```

To upgrade:

```bash
brew upgrade ado-cli
```

### From Source

```bash
git clone https://github.com/SammyLin/ado-cli.git
cd ado-cli
cargo install --path .
```

## Configuration

dotenvy auto-loads a `.env` from the cwd or any parent directory (`~/.env` works globally). Required keys:

```bash
ADO_ORG=MyOrg
ADO_PROJECT=MyProject
ADO_TEAM="My Team"        # quote values with spaces
ADO_PAT=<your PAT>
```

Create / regenerate PATs at `https://dev.azure.com/<ORG>/_usersSettings/tokens`.

| Variable | Description | Required |
|----------|-------------|----------|
| `ADO_ORG` | Azure DevOps organization name | ✅ |
| `ADO_PROJECT` | Project name | ✅ |
| `ADO_TEAM` | Team name (used for sprint/iteration queries) | ✅ |
| `ADO_PAT` | Personal Access Token | ✅ |

### PAT Scopes

- **Read-only** (`iterations`, `sprint list`, `item show`, `item list`, `item comment list`) — `Work Items: Read`
- **Write** (`item create`, `item update`, `item delete`, `item comment add/update/delete`) — `Work Items: Read & Write`

## Usage

### Iterations

```bash
ado-cli iterations                              # list all iterations
ado-cli iterations --json                       # JSON output
```

### Sprint Board

```bash
ado-cli sprint list                             # current sprint cards
ado-cli sprint list --iteration 'Sprint 2'      # named sprint
ado-cli sprint list --json | jq …               # pipe to jq
```

### Work Items

```bash
# Show
ado-cli item show 1196                          # description + acceptance criteria
ado-cli item show 1196 --json                   # raw JSON

# List with filters
ado-cli item list --assignee user@example.com --state Active
ado-cli item list --type Task --state Closed
ado-cli item list --iteration 'MyProject\Sprint 1'

# Create
ado-cli item create --parent 1197 --type Task \
  --assignee user@example.com --priority 2 \
  --title 'Implement login dialog'

# Batch create
ado-cli item create --parent 1197 --type Task \
  --assignee user@example.com \
  --title 'T1' --title 'T2' --title 'T3'

# Update
ado-cli item update 1202 --state Done
ado-cli item update 1202 --state Done --comment "Done — see commit 71ae422"
ado-cli item update 1202 --assignee user@example.com --priority 1

# Delete (moves to recycle bin)
ado-cli item delete 1205
```

### Comments

```bash
ado-cli item comment list 1196                                      # list all
ado-cli item comment add 1197 --text 'Done.'                        # add
ado-cli item comment update 1197 --comment-id 42 --text 'Updated'  # update
ado-cli item comment delete 1197 --comment-id 42                    # delete
```

### Links

```bash
ado-cli item link list 1196                                         # list all links
ado-cli item link add 1196 --target 1200 --type related             # add related link
ado-cli item link add 1196 --target 1200 --type child               # add child link
ado-cli item link add 1196 --target 1200 --type parent              # add parent link
ado-cli item link add 1196 --target 1200 --type related --comment 'See also'  # with comment
ado-cli item link remove 1196 --target 1200 --type related          # remove link
```

Available link types: `parent`, `child`, `related`, `duplicate`, `duplicate-of`, `predecessor`, `successor`.

### Notes on `--assignee`

ADO requires the `uniqueName` (e.g. `user@example.com`), not the display name. To find it:

```bash
ado-cli item show <id> --json | jq '.fields["System.AssignedTo"].uniqueName'
```

### Notes on `--parent`

When `--parent` is given, the new item inherits `System.IterationPath` and `System.AreaPath` from the parent unless `--iteration` / `--area` are passed explicitly.

## Directory Structure

```
ado-cli/
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── client.rs            # HTTP client, URL builders, auth
│   ├── config.rs            # Config from env / .env
│   └── commands/
│       ├── mod.rs            # Shared utilities (strip_html)
│       ├── iterations.rs     # ado-cli iterations
│       ├── sprint.rs         # ado-cli sprint list
│       ├── item.rs           # ado-cli item show/create/update/delete/list
│       ├── comment.rs        # ado-cli item comment list/add/update/delete
│       └── link.rs           # ado-cli item link list/add/remove
├── .github/workflows/
│   ├── test.yml              # CI: cargo test on push/PR
│   └── release.yml           # Release: cross-compile + homebrew tap
├── Cargo.toml
├── SKILL.md                  # AI assistant skill reference
├── .env.example
└── README.md
```

## Commands

| Command | Description |
|---------|-------------|
| `ado-cli iterations` | List iterations with timeframe and dates |
| `ado-cli sprint list` | List work items in current sprint |
| `ado-cli sprint list --iteration <name>` | List work items in a named sprint |
| `ado-cli item show <id>` | Show work item details |
| `ado-cli item list [--assignee/--state/--type/--iteration]` | Search work items with filters |
| `ado-cli item create --title <t> [opts]` | Create work item(s) |
| `ado-cli item update <id> [--state/--assignee/...]` | Update work item fields |
| `ado-cli item delete <id>` | Delete work item (recycle bin) |
| `ado-cli item comment list <id>` | List comments |
| `ado-cli item comment add <id> --text <t>` | Add a comment |
| `ado-cli item comment update <id> --comment-id <n> --text <t>` | Update a comment |
| `ado-cli item comment delete <id> --comment-id <n>` | Delete a comment |
| `ado-cli item link list <id>` | List links on a work item |
| `ado-cli item link add <id> --target <n> --type <t>` | Add a link to another work item |
| `ado-cli item link remove <id> --target <n> --type <t>` | Remove a link |

All read/write commands accept `--json` for machine-readable output.

## License

MIT License
