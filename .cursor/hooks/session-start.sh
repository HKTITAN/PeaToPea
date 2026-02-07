#!/bin/sh
# Cursor sessionStart hook: inject .tasks context so the agent prefers task-driven work.
# Receives JSON on stdin; must consume it. Outputs JSON with additional_context.
# See: https://cursor.com/docs/agent/hooks

cat > /dev/null

CONTEXT='PeaToPea: Work is driven by .tasks/ (see .tasks/README.md). When no specific goal is set, continue from the next unchecked item in order (00 → 01 → 07 → 02 & 03 → …). Proceed directly without asking for confirmation. After each logical chunk (e.g. a task item completed, a section done): run cargo build -p pea-core and cargo test -p pea-core if Rust changed, then git add, git commit with a clear present-tense message, and git push. Do not ask for confirmation to commit or push. Add new requirements to the appropriate .tasks file; record findings in a ## Notes section or inline (Note: …). When you finish a chunk, the session will auto-continue to the next task.'
printf '%s\n' "{\"additional_context\": \"$CONTEXT\"}"
exit 0
