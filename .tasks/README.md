# PeaPod Task Breakdown

Hierarchical task list for the full PeaPod project. Each file is a checklist; complete tasks in dependency order (e.g. core before protocol implementations).

## Task Files Index

| File | Scope |
|------|--------|
| [00-project-setup.md](00-project-setup.md) | Repo structure, tooling, CI, protocol versioning |
| [01-pea-core.md](01-pea-core.md) | PeaPod protocol logic and reference implementation: identity, protocol, chunking, scheduler, integrity, API, tests |
| [02-windows.md](02-windows.md) | Protocol implementation for Windows: proxy/WinDivert, discovery, transport, UI, installer |
| [03-android.md](03-android.md) | Protocol implementation for Android: VPNService, JNI, discovery, transport, UI |
| [04-linux.md](04-linux.md) | Protocol implementation for Linux: daemon, proxy/netfilter, discovery, systemd, packaging |
| [05-ios.md](05-ios.md) | Protocol implementation for iOS: Network Extension, Swift, Rust core, discovery, App Store |
| [06-macos.md](06-macos.md) | Protocol implementation for macOS: Network Extension, Swift, discovery, menu bar |
| [07-protocol-and-interop.md](07-protocol-and-interop.md) | Wire format, discovery spec, versioning, cross-platform interop tests |
| [08-documentation.md](08-documentation.md) | Architecture, protocol, build/run per platform |
| [09-quality-and-metrics.md](09-quality-and-metrics.md) | PRD success metrics, edge cases, risk mitigations |

## Recommended Order

1. **00-project-setup** — Get repo and CI in place.
2. **01-pea-core** — Must complete before any protocol implementation.
3. **07-protocol-and-interop** — Define wire format and discovery early; implement in core.
4. **02-windows** and **03-android** — First two implementations (validate interop).
5. **04-linux**, **06-macos**, **05-ios** — Remaining implementations.
6. **08-documentation** — In parallel; finalize with **09-quality-and-metrics**.

## Checklist Convention

- `- [ ]` = not done  
- `- [x]` = done  
- Use headers: `##` task, `###` subtask, `####` sub-subtask.
