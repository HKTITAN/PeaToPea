---
name: peapod-tasks
description: Use when implementing features or fixes that correspond to .tasks checklists. Open the relevant .tasks file (e.g. 01-pea-core, 02-windows), implement unchecked items, and mark completed items with [x]. Run fully autonomously: no confirmation for build, test, commit, or push.
---

# PeaPod Task-Driven Implementation

**Autonomous**: Do not ask for confirmation. Execute each task item, run build/test if Rust changed, commit and push, then continue to the next item. Hooks auto-allow shell commands and auto-continue the conversation.

When the user asks to implement work that maps to the task breakdown:

1. **Identify the task file** from [.tasks/README.md](.tasks/README.md). Follow the recommended order: 00 → 01 → 07 → 02 & 03 → 04, 05, 06 → 08, 09.

2. **Open the relevant .tasks file** (e.g. [.tasks/01-pea-core.md](.tasks/01-pea-core.md) for core work, [.tasks/02-windows.md](.tasks/02-windows.md) for Windows).

3. **Implement in dependency order**: Complete parent tasks before children where the checklist implies it. Do not skip items that block others.

4. **Update checklists**: As you complete each item, change `- [ ]` to `- [x]` in the corresponding .tasks file. Only mark done when the work is implemented and verified (e.g. build/tests pass).

5. **Add sub-tasks only when needed**: If you discover a necessary step not in the list, add it under the appropriate ## or ### so the hierarchy stays clear.

6. **Evolve .tasks**: When new requirements emerge, add tasks to the appropriate file; when findings arise (design decisions, root causes, gotchas), add notes inline or in a `## Notes` section at the end of the file. For session behavior (e.g. continuing from the next unchecked item when no specific goal is set), follow the task-continuity rule in `.cursor/rules/task-continuity.mdc`.
