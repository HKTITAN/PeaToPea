# Contributing to PeaPod (Project PeaToPea)

## Branching

- **main** — Stable, releasable state. Default branch.
- **develop** — Integration branch for ongoing work (optional; you can use only `main` and short-lived feature branches).
- **feature/&lt;name&gt;** — New work (e.g. `feature/pea-core-chunk-manager`, `feature/windows-proxy`). Merge into `develop` or `main` after review and passing tests.

Create feature branches from `main` (or `develop` if you use it). Keep branches short-lived and delete after merge.

## Habits

- **Commit often**: Small, logical commits with clear messages. Commit before switching task or at end of session.
- **Keep .cursor useful**: When you or the agent notice a repeated pattern or a new workflow, add or update a rule, skill, or subagent so future sessions benefit. Check in .cursor changes with the rest of the code.

## Commits

- Use clear, present-tense messages (e.g. "Add chunk manager to pea-core", "Fix frame decode for partial reads").
- Reference .tasks or issues when applicable (e.g. "Complete 01-pea-core section 4.2").

## Before pushing

1. Run `cargo build -p pea-core` and `cargo test -p pea-core` from the repo root.
2. If you changed pea-windows or pea-linux, run `cargo build` and `cargo test` for the full workspace.
3. Fix any build or test failures before pushing.

## Cursor

- **Rules**: Project conventions live in `.cursor/rules/`. They guide style, terminology, and workflow.
- **Skills**: Use the build-test and task-driven skills when implementing; use the platform-impl skill when working on Windows/Android/Linux/iOS/macOS code.
- **Verifier**: Before marking tasks done, invoke the verifier subagent (e.g. `/verifier`) to confirm builds and tests pass.
- **Hooks (fully autonomous)**: Agent hooks are in [.cursor/hooks.json](.cursor/hooks.json) and [.cursor/hooks/](.cursor/hooks/). `sessionStart` injects .tasks context. `stop` auto-submits a "continue with next task" message (Cursor enforces max 5 auto-continuations per conversation; after that the agent is asked to summarize and stop). `beforeShellExecution` auto-allows shell commands (cargo, git) so you are never prompted to approve. `subagentStart` auto-allows subagent (Task tool) dispatch. Together with rules and skills, the workflow runs without user confirmation. Python 3 is required for `.cursor/hooks/stop.py`. See [Cursor Hooks](https://cursor.com/docs/agent/hooks).

## Tasks

- Work is driven by [.tasks/](.tasks/); follow the order in [.tasks/README.md](.tasks/README.md).
- When you discover new requirements, add them to the appropriate .tasks file.
- When you have findings or design notes, add them inline or in a `## Notes` section in the relevant task file.
- The pre-push hook reminds you to check .tasks; Cursor rules and the peapod-tasks skill guide the agent to continue from .tasks and to update them.

## GitHub: private repo PeaToPea

This repo is intended to be pushed to a **private** GitHub repository named **PeaToPea**.

### Option A: GitHub CLI (`gh`)

If you have [GitHub CLI](https://cli.github.com/) installed and authenticated:

```bash
cd c:\.projects\PeaToPea
gh repo create PeaToPea --private --source=. --remote=origin --push
```

This creates the private repo, adds `origin`, and pushes the current branch.

### Option B: GitHub web

1. On [GitHub](https://github.com/new): **New repository** → Name: **PeaToPea** → **Private** → do **not** add README, .gitignore, or license (this repo already has them).
2. After the first local commit and branch setup:

```bash
git remote add origin https://github.com/<your-username>/PeaToPea.git
git push -u origin main
git push -u origin develop   # if you created develop
```

Replace `<your-username>` with your GitHub username or org.

### Remote

- **Remote name**: `origin`
- **URL**: `https://github.com/<your-username>/PeaToPea.git` (or SSH: `git@github.com:<your-username>/PeaToPea.git`)
