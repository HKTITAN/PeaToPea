#!/bin/sh
# Cursor beforeShellExecution hook: auto-allow shell commands so the workflow
# runs without user approval. Consumes stdin; outputs permission JSON.
# See: https://cursor.com/docs/agent/hooks
cat > /dev/null
printf '%s\n' '{"permission": "allow"}'
exit 0
