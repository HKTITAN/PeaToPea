---
name: peapod-build-test
description: Use when building, testing, or verifying pea-core or the workspace. Run cargo build and cargo test for pea-core from repo root; fix compilation or test failures. Execute without asking for confirmation (autonomous mode).
---

# PeaPod Build and Test

**Autonomous**: Run build and test commands without asking for confirmation. Hooks allow shell execution automatically.

When building or verifying the project:

1. **From the repo root**, run:
   - `cargo build -p pea-core`
   - `cargo test -p pea-core`

2. **Report**: State whether build and tests passed or failed. If they failed, fix the cause (compile errors, failing tests) and re-run until both pass.

3. **Full workspace** (optional): When working on pea-windows or pea-linux too, run `cargo build` and `cargo test` at the root to build and test all workspace members.

4. **Scripts**: Optional helper scripts are in this skill's `scripts/` folder if present (e.g. `build-and-test.ps1` for Windows). Prefer running the commands above directly when the agent can execute them.
