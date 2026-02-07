#!/usr/bin/env python3
"""
Cursor stop hook: auto-continue development by submitting a follow-up message
so the agent proceeds to the next .tasks item without asking the user.
See: https://cursor.com/docs/agent/hooks (stop hook, followup_message)
"""
import json
import sys

# Cursor enforces max 5 auto follow-ups per conversation; we cannot exceed that.
MAX_AUTO_CONTINUATIONS = 5

CONTINUE_MESSAGE = (
    "Fully autonomous: do not ask for confirmation. If you have uncommitted changes: run cargo build -p pea-core and cargo test -p pea-core if Rust changed, then git add, git commit with a clear message, and git push. "
    "Then continue with the next unchecked item in .tasks (see .tasks/README.md). "
    "If all tasks in the current file are done, move to the next task file in order (00 → 01 → 07 → 02 & 03 → …)."
)
LIMIT_MESSAGE = (
    "Auto-continuation limit reached (Cursor allows 5 per conversation). Briefly summarize what was completed and what is next in .tasks, then stop."
)


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        print("{}")
        return

    status = payload.get("status", "")
    loop_count = payload.get("loop_count", 0)

    # Only auto-continue when the agent completed or hit an error (not when user aborted)
    if status == "aborted":
        print("{}")
        return

    # Cursor enforces max 5; when we hit it, send a clear "summarize and stop" so the agent hands off cleanly
    if loop_count >= MAX_AUTO_CONTINUATIONS:
        out = {"followup_message": LIMIT_MESSAGE}
    else:
        out = {"followup_message": CONTINUE_MESSAGE}

    print(json.dumps(out))


if __name__ == "__main__":
    main()
