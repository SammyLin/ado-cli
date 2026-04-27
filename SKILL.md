---
name: ado-cli
description: Query AND mutate Azure DevOps work items (sprint cards, iterations, single items with description + acceptance criteria, plus create / comment) via the local `ado` CLI. Use when the user asks about "sprint 卡片" / "ADO 卡片" / "current sprint" / "iteration" / "work item #N" / a `dev.azure.com` URL, OR wants to break a story into Tasks ("拆 task" / "建 task under #N"), OR add/read a comment on a card. The `ado` binary wraps the ADO REST API and reads `ADO_ORG`/`ADO_PROJECT`/`ADO_TEAM`/`ADO_PAT` from env or a `.env` file in the cwd (or any parent dir).
---

# ado-cli skill

The `ado` CLI lives at `~/.cargo/bin/ado` (installed via `cargo install --path ~/workspace/ado-cli`). It wraps the Azure DevOps REST API so you don't have to hand-roll WIQL queries, json-patch bodies, or base64-encode PATs.

## When to use this skill

Trigger on any of these signals:

**Read:**
- User mentions "sprint 卡片", "ADO 卡片", "current sprint", "iteration"
- User pastes a `dev.azure.com/.../_sprints/...` or `_workitems/edit/<id>` URL
- User asks for a specific work item's description / AC ("看一下 #1196")
- User asks "what's left in the sprint" / "Sprint N 有什麼"

**Write:**
- "幫我建 task under #N" / "把 #N 拆成幾張 task" / "create a Task / Bug under …"
- "把 #N 改成 Done" / "close #N" / "set #N to Active" / "reassign #N to …"
- "在 #N 留個 comment" / "add a comment to #N" / "看一下 #N 的留言 / comment list"

Skip this skill if:
- The user is on a project not configured in `~/.env` / env vars (the `ADO_*` vars target one specific org+project+team).

## Prerequisites — verify before invoking

1. Binary exists: `which ado` should print `/Users/<user>/.cargo/bin/ado`. If missing: `cd ~/workspace/ado-cli && cargo install --path . --force`.
2. Env vars set. Either:
   - Persistent: a `.env` in the cwd OR a parent directory (typically `~/.env`) with `ADO_ORG` / `ADO_PROJECT` / `ADO_TEAM` / `ADO_PAT` (see `~/workspace/ado-cli/.env.example`), OR
   - Per-call: prefix the command with `ADO_PAT='…' ado …`
   - **Quote values that contain spaces**: `ADO_TEAM="My Team"`. dotenvy without quotes can mis-parse the team name.
3. The PAT must be live AND have the right scope:
   - **Read-only commands** (`iterations`, `sprint list`, `item show`, `item comment list`) — `Work Items: Read` is enough.
   - **Write commands** (`item create`, `item update`, `item comment add`) — need `Work Items: Read & Write`.
   - On 401 ADO returns an HTML page titled `Personal Access Token used has expired` — the CLI extracts the title into the error so the cause is obvious.
   - Regen at `https://dev.azure.com/<ORG>/_usersSettings/tokens`.

## Commands

### Read

#### `ado iterations [--json]`
Lists every iteration the team can see, with `timeFrame` (`past` / `current` / `future`) and start/finish dates. Use this first if you're unsure which iteration the user means, or to discover sprint paths.

#### `ado sprint list [--iteration <NAME|FULL\PATH>] [--json]`
Lists work items in an iteration. Defaults to whichever iteration has `timeFrame=current`. The `--iteration` flag accepts either:
- the bare name: `--iteration 'i-Sprint 2'`
- the full path: `--iteration 'MyProject\Sprint 1\i-Sprint 2'`

Output columns: `ID | TYPE | STATE | SP | PRI | ASSIGNEE | TITLE`. Use `--json` for jq pipelines.

#### `ado item show <id> [--json]`
Shows a single work item with its description and acceptance criteria (HTML stripped to plain text). Use this after `sprint list` when the user wants details on a specific card. Use `--json` if you need raw fields (relations, identity descriptors, etc.).

#### `ado item comment list <id> [--json]`
Lists comments on a work item, oldest first.

### Write

#### `ado item create --title <T> [--title <T>...] [options]`
Creates one or more work items. Repeat `--title` to create multiple in one call (each gets its own work item, all share the other args).

Options:
- `--type <TYPE>` — work item type. Default `Task`. Examples: `Task`, `Bug`, `"User Story"`, `Epic`.
- `--parent <ID>` — parent work item id. **Inherits the parent's iteration + area** unless overridden. Use this 99% of the time when breaking a story into tasks.
- `--assignee <UNIQUE_NAME>` — must be `uniqueName` format, e.g. `user@example.com` (NOT display name). To find someone's uniqueName, run `ado item show <ID> --json | jq '.fields["System.AssignedTo"].uniqueName'` on a card they're already assigned to.
- `--priority <N>` — 1=highest, 4=lowest. Default = ADO field default.
- `--description <TEXT>` — plain text or HTML. Applied to every created item.
- `--iteration <PATH>` — overrides parent's iteration.
- `--area <PATH>` — overrides parent's area.
- `--json` — emit raw JSON for the created items.

#### `ado item update <id> [options]`
Updates fields on an existing work item. At least one field flag is required.

Options:
- `--state <STATE>` — transition the work item state (e.g. `New` → `Active` → `Done` → `Closed`). ADO enforces workflow rules — invalid transitions return HTTP 400 with a `TF401320` error.
- `--assignee <UNIQUE_NAME>` — reassign (same `uniqueName` format as create).
- `--priority <N>` — 1=highest, 4=lowest.
- `--title <STR>` — change the title.
- `--iteration <PATH>` — move to a different iteration.
- `--description <STR>` — replace description (HTML allowed).
- `--field <name=value>` — repeatable escape hatch for any field, e.g. `--field 'Custom.MyField=foo'`.
- `--comment <STR>` — also append a comment after the update succeeds (reuses existing comment logic).
- `--json` — emit the raw PATCH response.

#### `ado item comment add <id> --text "<TEXT>"`
Adds one comment. ADO accepts plain text or HTML. Use this for audit trails when breaking down a story (link the new task IDs back to the parent).

## Recipes

**"Close #1202 with audit comment"**
```
ado item update 1202 --state Done --comment "Done — see commits afc9d42, f536e55, 71ae422"
```

**"What cards are in the current sprint?"**
```
ado sprint list
```

**"Show me #1196's description"**
```
ado item show 1196
```

**"Group sprint cards by assignee"**
```
ado sprint list --json | jq 'group_by(.fields["System.AssignedTo"].displayName) | map({assignee: .[0].fields["System.AssignedTo"].displayName, count: length, ids: map(.id)})'
```

**"Break #1197 into 4 sub-Tasks, all assigned to Sammy"**

Always do this in two halves: (1) create the tasks, capturing IDs from output; (2) post one comment on the parent that lists the IDs as audit trail.

```
ado item create --parent 1197 --type Task --assignee user@example.com --priority 2 \
  --title 'T1: ...' --description '...'
ado item create --parent 1197 --type Task --assignee user@example.com --priority 2 \
  --title 'T2: ...' --description '...'
# … repeat per task with its own description …

ado item comment add 1197 --text 'Broken into T1 (#1200) ... T4 (#1203). Reasons: ...'
```

**Why one create per task?** `--title` is repeatable for batch, but every title in a single call shares the same description/assignee/priority. When per-task descriptions differ (which is the common case), call `ado item create` once per task.

**"Add a comment to #1197"**
```
ado item comment add 1197 --text 'Schema for tenant_questionnaires.final_deadline merged in commit abc123.'
```

**"List Sprint 2 cards"**
```
ado sprint list --iteration 'i-Sprint 2'
```

## Workflow: breaking a User Story into Tasks

When the user asks to "拆 #N into tasks":

1. **Read the story first.** `ado item show <N>` to get description + AC. Do NOT propose tasks based on the title alone.
2. **Cross-check against actual code.** If a CLAUDE.md / PRD claims the feature is "done", verify in the codebase before listing tasks for already-implemented work. The user wants the *real* gap, not a re-tracing of the AC.
3. **Confirm the breakdown with the user before creating.** Ask: (a) is the proposed list right? (b) who's the assignee? (c) leave a comment on the parent? Wait for answers.
4. **Find the assignee's `uniqueName`.** Display name like "John.Doe" is NOT enough — you need `user@example.com`. If unsure, ask the user explicitly or grep an existing card's JSON.
5. **Create one at a time.** Each call returns `created #ID title` — record the IDs.
6. **Leave a comment on the parent.** List the new task IDs and the rationale (especially if you skipped any items from the original AC because they were already done).

## Failure modes & how to react

| Error | Cause | Fix |
|---|---|---|
| `HTTP 401: Access Denied: The Personal Access Token used has expired.` | PAT expired or wrong scope | Regen at `dev.azure.com/<ORG>/_usersSettings/tokens` (need R/W for create/comment) and update `~/.env` |
| `missing env var ADO_TEAM` (or any) | dotenvy didn't load, or value with spaces wasn't quoted | Quote: `ADO_TEAM="My Team"`. Verify with `grep '^ADO_' ~/.env` (4 lines expected) |
| `iteration not found: …` | Typo in `--iteration` | Run `ado iterations` to list valid names/paths |
| `no iteration with timeFrame=current` | No active sprint configured for the team | Pick a specific iteration with `--iteration` |
| `HTTP 400: TF401320: Rule Error for field …` | Required field missing on create (e.g. iteration not inheriting because parent omitted) | Pass `--iteration` and `--area` explicitly; `--parent` only inherits if the parent has those fields set |
| `HTTP 400: VS403072: The user … is not a valid user` | Wrong assignee format — used display name instead of uniqueName | Use `user@example.com` format (e.g. `user@example.com`). Find via `ado item show <existing-id> --json | jq '.fields["System.AssignedTo"].uniqueName'` |

## Out of scope (do NOT improvise)

- Deleting existing work items — only `create`, `update`, and `comment add` are implemented. If the user asks to delete a work item, say so and offer to extend `~/workspace/ado-cli`.
- Cross-team queries — `ADO_TEAM` targets a single team. To query another team, override the env var for that call.
- Attachments, history — not implemented. Source at `~/workspace/ado-cli` if needed.
