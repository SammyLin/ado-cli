---
name: ado-cli
description: Query AND mutate Azure DevOps work items (sprint cards, iterations, single items with description + acceptance criteria, plus create / update / delete / list / comment CRUD / link management) via the local `ado` CLI. Use when the user asks about "sprint 卡片" / "ADO 卡片" / "current sprint" / "iteration" / "work item #N" / a `dev.azure.com` URL, OR wants to break a story into Tasks ("拆 task" / "建 task under #N"), OR update/close/delete a card, OR add/read a comment on a card, OR link/unlink work items. The `ado` binary wraps the ADO REST API and reads `ADO_ORG`/`ADO_PROJECT`/`ADO_TEAM`/`ADO_PAT` from env or a `.env` file in the cwd (or any parent dir).
---

# ado-cli skill

The `ado-cli` CLI lives at `~/.cargo/bin/ado-cli` (installed via `cargo install --path ~/workspace/ado-cli`). It wraps the Azure DevOps REST API so you don't have to hand-roll WIQL queries, json-patch bodies, or base64-encode PATs.

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
- "把 #N 跟 #M 連起來" / "link #N to #M" / "add related link" / "remove link"

Skip this skill if:
- The user is on a project not configured in `~/.env` / env vars (the `ADO_*` vars target one specific org+project+team).

## Prerequisites — verify before invoking

1. Binary exists: `which ado-cli` should print `/Users/<user>/.cargo/bin/ado-cli`. If missing: `cd ~/workspace/ado-cli && cargo install --path . --force`.
2. Config set. Either:
   - **Preferred**: a `.ado.toml` in the cwd or any parent directory. Run `ado-cli init` to create one interactively.
   - **Fallback**: env vars `ADO_ORG` / `ADO_PROJECT` / `ADO_TEAM` / `ADO_PAT` (via shell or `.env` file).
   - Each field falls back independently: `.ado.toml` fields take priority, missing fields fall back to env vars.
3. The PAT must be live AND have the right scope:
   - **Read-only commands** (`iterations`, `sprint list`, `item show`, `item comment list`) — `Work Items: Read` is enough.
   - **Write commands** (`item create`, `item update`, `item comment add`) — need `Work Items: Read & Write`.
   - On 401 ADO returns an HTML page titled `Personal Access Token used has expired` — the CLI extracts the title into the error so the cause is obvious.
   - Regen at `https://dev.azure.com/<ORG>/_usersSettings/tokens`.

## Commands

### Read

#### `ado-cli iterations [--json]`
Lists every iteration the team can see, with `timeFrame` (`past` / `current` / `future`) and start/finish dates. Use this first if you're unsure which iteration the user means, or to discover sprint paths.

#### `ado-cli sprint list [--iteration <NAME|FULL\PATH>] [--json]`
Lists work items in an iteration. Defaults to whichever iteration has `timeFrame=current`. The `--iteration` flag accepts either:
- the bare name: `--iteration 'i-Sprint 2'`
- the full path: `--iteration 'MyProject\Sprint 1\i-Sprint 2'`

Output columns: `ID | TYPE | STATE | SP | PRI | ASSIGNEE | TITLE`. Use `--json` for jq pipelines.

#### `ado-cli item show <id> [--json]`
Shows a single work item with its description and acceptance criteria (HTML stripped to plain text). Use this after `sprint list` when the user wants details on a specific card. Use `--json` if you need raw fields (relations, identity descriptors, etc.).

#### `ado-cli item list [--assignee <EMAIL>] [--state <STATE>] [--type <TYPE>] [--iteration <PATH>] [--json]`
Search work items using WIQL with optional filters. All filters are AND-combined. Without filters, lists all items in the project (descending by ID).

Examples:
- `ado-cli item list --assignee user@example.com --state Active` — my active items
- `ado-cli item list --type Task --state Closed` — all closed tasks
- `ado-cli item list --iteration 'MyProject\Sprint 1'` — items under an iteration (uses UNDER, so sub-iterations match too)

#### `ado-cli item comment list <id> [--json]`
Lists comments on a work item, oldest first.

### Write

#### `ado-cli item create --title <T> [--title <T>...] [options]`
Creates one or more work items. Repeat `--title` to create multiple in one call (each gets its own work item, all share the other args).

Options:
- `--type <TYPE>` — work item type. Default `Task`. Examples: `Task`, `Bug`, `"User Story"`, `Epic`.
- `--parent <ID>` — parent work item id. **Inherits the parent's iteration + area** unless overridden. Use this 99% of the time when breaking a story into tasks.
- `--assignee <UNIQUE_NAME>` — must be `uniqueName` format, e.g. `user@example.com` (NOT display name). To find someone's uniqueName, run `ado-cli item show <ID> --json | jq '.fields["System.AssignedTo"].uniqueName'` on a card they're already assigned to.
- `--priority <N>` — 1=highest, 4=lowest. Default = ADO field default.
- `--description <TEXT>` — plain text or HTML. Applied to every created item.
- `--iteration <PATH>` — overrides parent's iteration.
- `--area <PATH>` — overrides parent's area.
- `--json` — emit raw JSON for the created items.

#### `ado-cli item update <id> [options]`
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

#### `ado-cli item delete <id>`
Deletes a work item (moves to recycle bin). Use when a task was created by mistake.

#### `ado-cli item comment add <id> --text "<TEXT>"`
Adds one comment. ADO accepts plain text or HTML. Use this for audit trails when breaking down a story (link the new task IDs back to the parent).

#### `ado-cli item comment update <id> --comment-id <N> --text "<TEXT>"`
Updates an existing comment's text.

#### `ado-cli item comment delete <id> --comment-id <N>`
Deletes a comment.

#### `ado-cli item link list <id> [--json]`
Lists all links (relations) on a work item — parent/child, related, duplicates, etc. Non-work-item links (hyperlinks, artifacts) are also shown.

#### `ado-cli item link add <id> --target <TARGET_ID> --type <TYPE> [--comment <TEXT>] [--json]`
Adds a link between two work items.

Available link types: `parent`, `child`, `related`, `duplicate`, `duplicate-of`, `predecessor`, `successor`.

- `--comment` — optional comment on the link (visible in ADO's link detail).

#### `ado-cli item link remove <id> --target <TARGET_ID> --type <TYPE>`
Removes a link. The CLI fetches the work item's relations, finds the matching link by type + target URL, and issues a `remove` patch at the correct index.

#### `ado-cli item link add-commit <id> --repo <REPO_NAME> --commit <SHA> [--comment <TEXT>] [--json]`
Links a Git commit to a work item. The CLI resolves the repo name to its GUID via the ADO Git API, then creates an `ArtifactLink` relation with the `vstfs:///Git/Commit/...` URL. You only need the repo name and commit SHA.

## Recipes

**"Close #1202 with audit comment"**
```
ado-cli item update 1202 --state Done --comment "Done — see commits afc9d42, f536e55, 71ae422"
```

**"Find all my active items"**
```
ado-cli item list --assignee user@example.com --state Active
```

**"Delete a mistakenly created task"**
```
ado-cli item delete 1205
```

**"What cards are in the current sprint?"**
```
ado-cli sprint list
```

**"Show me #1196's description"**
```
ado-cli item show 1196
```

**"Group sprint cards by assignee"**
```
ado-cli sprint list --json | jq 'group_by(.fields["System.AssignedTo"].displayName) | map({assignee: .[0].fields["System.AssignedTo"].displayName, count: length, ids: map(.id)})'
```

**"Break #1197 into 4 sub-Tasks, all assigned to Sammy"**

Always do this in two halves: (1) create the tasks, capturing IDs from output; (2) post one comment on the parent that lists the IDs as audit trail.

```
ado-cli item create --parent 1197 --type Task --assignee user@example.com --priority 2 \
  --title 'T1: ...' --description '...'
ado-cli item create --parent 1197 --type Task --assignee user@example.com --priority 2 \
  --title 'T2: ...' --description '...'
# … repeat per task with its own description …

ado-cli item comment add 1197 --text 'Broken into T1 (#1200) ... T4 (#1203). Reasons: ...'
```

**Why one create per task?** `--title` is repeatable for batch, but every title in a single call shares the same description/assignee/priority. When per-task descriptions differ (which is the common case), call `ado-cli item create` once per task.

**"Add a comment to #1197"**
```
ado-cli item comment add 1197 --text 'Schema for tenant_questionnaires.final_deadline merged in commit abc123.'
```

**"Link #1196 as related to #1200"**
```
ado-cli item link add 1196 --target 1200 --type related --comment 'See also'
```

**"Make #1200 a child of #1196"**
```
ado-cli item link add 1196 --target 1200 --type child
```

**"Remove the related link between #1196 and #1200"**
```
ado-cli item link remove 1196 --target 1200 --type related
```

**"List all links on #1196"**
```
ado-cli item link list 1196
```

**"Link a commit to #1225"**
```
ado-cli item link add-commit 1225 --repo ifrs-web --commit abc123def --comment 'fix: login dialog'
```

**"List Sprint 2 cards"**
```
ado-cli sprint list --iteration 'i-Sprint 2'
```

## Workflow: breaking a User Story into Tasks

When the user asks to "拆 #N into tasks":

1. **Read the story first.** `ado-cli item show <N>` to get description + AC. Do NOT propose tasks based on the title alone.
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
| `iteration not found: …` | Typo in `--iteration` | Run `ado-cli iterations` to list valid names/paths |
| `no iteration with timeFrame=current` | No active sprint configured for the team | Pick a specific iteration with `--iteration` |
| `HTTP 400: TF401320: Rule Error for field …` | Required field missing on create (e.g. iteration not inheriting because parent omitted) | Pass `--iteration` and `--area` explicitly; `--parent` only inherits if the parent has those fields set |
| `HTTP 400: VS403072: The user … is not a valid user` | Wrong assignee format — used display name instead of uniqueName | Use `user@example.com` format (e.g. `user@example.com`). Find via `ado-cli item show <existing-id> --json | jq '.fields["System.AssignedTo"].uniqueName'` |

## Out of scope (do NOT improvise)

- Cross-team queries — `ADO_TEAM` targets a single team. To query another team, override the env var for that call.
- Attachments, history — not implemented. Source at `~/workspace/ado-cli` if needed.
