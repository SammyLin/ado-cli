# ado-cli

[![Tests](https://github.com/SammyLin/ado-cli/actions/workflows/test.yml/badge.svg)](https://github.com/SammyLin/ado-cli/actions/workflows/test.yml)
[![Rust](https://img.shields.io/badge/Rust-2021-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Tiny Rust CLI for querying and mutating Azure DevOps work items (sprint cards, iterations, single items, comments).

## Install

```bash
cargo install --path .
# binary lands at ~/.cargo/bin/ado
```

## Configure

dotenvy auto-loads a `.env` from the cwd or any parent directory (so `~/.env` works for any path under home). Required keys:

```
ADO_ORG=MyOrg
ADO_PROJECT=MyProject
ADO_TEAM="My Team"               # quote values with spaces
ADO_PAT=<your PAT>
```

PAT scopes:
- **Read-only commands** (`iterations`, `sprint list`, `item show`, `item comment list`) — `Work Items: Read`.
- **Write commands** (`item create`, `item comment add`) — `Work Items: Read & Write`.

Create / regenerate at `https://dev.azure.com/<ORG>/_usersSettings/tokens`.

You can also pass everything inline:

```bash
ADO_PAT=… ado sprint list
```

## Usage

### Read

```bash
ado iterations                              # list iterations (past / current / future)
ado sprint list                             # cards in the current sprint
ado sprint list --iteration 'i-Sprint 2'    # cards in a named sprint
ado sprint list --json | jq …               # machine-readable
ado item show 1196                          # description + acceptance criteria
ado item show 1196 --json                   # raw JSON (for relations, identity, etc.)
ado item comment list 1196                  # all comments, oldest first
```

### Write

```bash
# Create a single Task under #1197, inheriting parent's iteration + area
ado item create --parent 1197 --type Task \
  --assignee user@example.com --priority 2 \
  --title 'T1: User 管理 dialog' \
  --description '後端 CRUD 完整，補前端 dialog'

# Batch: same description / assignee for all
ado item create --parent 1197 --type Task \
  --assignee user@example.com \
  --title 'T1' --title 'T2' --title 'T3'

# Comment
ado item comment add 1197 --text 'Broken into T1 (#1200) ... T4 (#1203).'
```

### Notes on `--assignee`

ADO requires the `uniqueName` (e.g. `user@example.com`), NOT the display name. To find someone's uniqueName, look at a card they're already on:

```bash
ado item show <id> --json | jq '.fields["System.AssignedTo"].uniqueName'
```

### Notes on `--parent`

When `--parent` is given, the new item inherits `System.IterationPath` and `System.AreaPath` from the parent unless `--iteration` / `--area` are passed explicitly. The parent link is created as `System.LinkTypes.Hierarchy-Reverse` (i.e. the new item appears as a child).

## Claude Code skill

A matching skill lives at `~/.claude/skills/ado-cli/SKILL.md` so Claude Code triggers on phrases like:

- Read: "sprint 卡片" / "ADO 卡片" / "看一下 #N" / "current sprint"
- Write: "拆 #N" / "建 task under #N" / "在 #N 留 comment"

It documents the recommended workflow for breaking a User Story into Tasks (read → audit code → confirm with user → create → comment).
