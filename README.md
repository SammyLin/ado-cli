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
ado iterations                              # list all iterations
ado iterations --json                       # JSON output
```

### Sprint Board

```bash
ado sprint list                             # current sprint cards
ado sprint list --iteration 'Sprint 2'      # named sprint
ado sprint list --json | jq …               # pipe to jq
```

### Work Items

```bash
# Show
ado item show 1196                          # description + acceptance criteria
ado item show 1196 --json                   # raw JSON

# List with filters
ado item list --assignee user@example.com --state Active
ado item list --type Task --state Closed
ado item list --iteration 'MyProject\Sprint 1'

# Create
ado item create --parent 1197 --type Task \
  --assignee user@example.com --priority 2 \
  --title 'Implement login dialog'

# Batch create
ado item create --parent 1197 --type Task \
  --assignee user@example.com \
  --title 'T1' --title 'T2' --title 'T3'

# Update
ado item update 1202 --state Done
ado item update 1202 --state Done --comment "Done — see commit 71ae422"
ado item update 1202 --assignee user@example.com --priority 1

# Delete (moves to recycle bin)
ado item delete 1205
```

### Comments

```bash
ado item comment list 1196                                      # list all
ado item comment add 1197 --text 'Done.'                        # add
ado item comment update 1197 --comment-id 42 --text 'Updated'  # update
ado item comment delete 1197 --comment-id 42                    # delete
```

### Notes on `--assignee`

ADO requires the `uniqueName` (e.g. `user@example.com`), not the display name. To find it:

```bash
ado item show <id> --json | jq '.fields["System.AssignedTo"].uniqueName'
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
│       ├── iterations.rs     # ado iterations
│       ├── sprint.rs         # ado sprint list
│       ├── item.rs           # ado item show/create/update/delete/list
│       └── comment.rs        # ado item comment list/add/update/delete
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
| `ado iterations` | List iterations with timeframe and dates |
| `ado sprint list` | List work items in current sprint |
| `ado sprint list --iteration <name>` | List work items in a named sprint |
| `ado item show <id>` | Show work item details |
| `ado item list [--assignee/--state/--type/--iteration]` | Search work items with filters |
| `ado item create --title <t> [opts]` | Create work item(s) |
| `ado item update <id> [--state/--assignee/...]` | Update work item fields |
| `ado item delete <id>` | Delete work item (recycle bin) |
| `ado item comment list <id>` | List comments |
| `ado item comment add <id> --text <t>` | Add a comment |
| `ado item comment update <id> --comment-id <n> --text <t>` | Update a comment |
| `ado item comment delete <id> --comment-id <n>` | Delete a comment |

All read/write commands accept `--json` for machine-readable output.

## License

MIT License
