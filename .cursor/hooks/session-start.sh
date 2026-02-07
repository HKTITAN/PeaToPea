#!/bin/sh
# Cursor sessionStart hook: inject .tasks context so the agent prefers task-driven work.
# Receives JSON on stdin; must consume it. Outputs JSON with additional_context.
# See: https://cursor.com/docs/agent/hooks

cat > /dev/null

CONTEXT='PeaToPea runs in fully autonomous mode. Do not ask the user for confirmation for any step. Work is driven by .tasks/ (see .tasks/README.md). When no specific goal is set, continue from the next unchecked item in order (00 → 01 → 07 → 02 & 03 → …). Hooks auto-allow shell commands (cargo, git) and subagent dispatch; the stop hook auto-continues the conversation. After each logical chunk: run cargo build -p pea-core and cargo test -p pea-core if Rust changed, then git add, git commit with a clear present-tense message, and git push. Add new requirements to the appropriate .tasks file; record findings in ## Notes or inline (Note: …).'
printf '%s\n' "{\"additional_context\": \"$CONTEXT\"}"
exit 0
