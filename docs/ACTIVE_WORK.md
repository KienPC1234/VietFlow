# Active Work & File Claims

Single coordination table for the main agent and every subagent. It prevents two
agents from editing the same files at the same time.

## Before editing

1. Read this table.
2. Add or update your row **before** touching any file.
3. Claim the narrowest paths you need (file or folder/glob).
4. If another active row claims an overlapping path, stop and coordinate. Do not
   edit that path until the row is released.
5. When finished, mark the row `done` or remove it.

## Claims

| Agent / session | Task | Files / paths claimed | Started (UTC) | Status |
| --- | --- | --- | --- | --- |
| updater session (owner not registered) | velopack update integration (`test_api` stub) | `platforms/windows/src/updater.rs`, `platforms/windows/src/main.rs` (`mod updater`), `platforms/windows/Cargo.toml` (`velopack`) | 2026-10-29T14:28:46Z | active — coordinate before editing shared files |

_No other active claims at the time of the last update (2026-10-29T07:35Z)._

## Rules

- Claims are advisory for humans but **mandatory for agents**.
- The main agent must check this table before dispatching work and must include
  the claiming requirement in every subagent prompt.
- A subagent may update only its own row. It must not rewrite another agent's row.
- A subagent must not edit files claimed by another active row. If overlap is
  unavoidable, the main agent serializes the work or reassigns the claim.
- Release a claim as soon as the task is finished. Do not keep a claim while idle.
- If a claim is older than 2 hours without progress, the main agent may clear it
  after confirming that no agent or process is still using those files.
- Keep claims specific. Prefer `ui/qml/pages/MacroPage.qml` over `ui/`.
- Never put secrets, tokens, customer data or full config contents in this file.
- Before a release commit, remove completed rows so the table only shows active work.
