---
name: commit-and-cursor
description: Use when finishing a chunk of work or when the user asks to commit or to update Cursor context. Commit changes with a clear message; optionally add or update a rule, skill, or subagent in .cursor if something new was learned or a pattern should be documented.
---

# Commit and Update Cursor

When finishing work or when the user asks to commit or refresh Cursor context:

1. **Ensure build/tests pass** if Rust or the workspace was changed: run `cargo build -p pea-core` and `cargo test -p pea-core` from the repo root (or full `cargo build` / `cargo test` if other crates were touched). Fix any failures before committing.

2. **Stage, commit, and push** with a clear, present-tense message. Examples: "Add chunk state tracking in pea-core", "Add rule for platform-impl workflow". Prefer one logical change per commit. Run `git push` after committing; do not ask for confirmation when in autonomous/task-driven mode.

3. **If a new pattern or workflow emerged**, add or edit a file in `.cursor/rules/`, `.cursor/skills/<name>/SKILL.md`, or `.cursor/agents/<name>.md`. Commit that in the same commit or a follow-up commit (e.g. "Add workflow-habits rule and document in CONTRIBUTING").
