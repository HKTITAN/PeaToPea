#!/bin/sh
# Cursor subagentStart hook: auto-allow subagent (Task tool) dispatch so the
# agent can spawn explore/shell/generalPurpose without user confirmation.
# See: https://cursor.com/docs/agent/hooks
cat > /dev/null
printf '%s\n' '{"decision": "allow"}'
exit 0
