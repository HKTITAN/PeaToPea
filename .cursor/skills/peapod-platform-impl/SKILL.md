---
name: peapod-platform-impl
description: Use when working on Windows, Android, Linux, iOS, or macOS protocol implementations (pea-windows, pea-android, etc.). Follow the corresponding .tasks file (02–06); ensure wire format and discovery match pea-core and .tasks/07.
---

# PeaPod Platform Implementation

When working on a protocol implementation for a specific OS:

1. **Reference the right .tasks file**: [.tasks/02-windows.md](.tasks/02-windows.md), [.tasks/03-android.md](.tasks/03-android.md), [.tasks/04-linux.md](.tasks/04-linux.md), [.tasks/05-ios.md](.tasks/05-ios.md), or [.tasks/06-macos.md](.tasks/06-macos.md). Implement the checklist items; do not put platform I/O (sockets, VPN, proxy) inside pea-core.

2. **Use pea-core as dependency**: The implementation crate (e.g. pea-windows) depends on pea-core. Call into pea-core for protocol logic, chunking, scheduling, and integrity; the implementation handles discovery, traffic interception, and local transport.

3. **Wire format and discovery**: Keep the same wire format and discovery protocol as defined in pea-core and [.tasks/07-protocol-and-interop.md](.tasks/07-protocol-and-interop.md). All platforms must interoperate (e.g. Windows and Android in the same pod).

4. **Platform-specific mechanisms**: Use the correct OS mechanism for traffic interception (e.g. system proxy or WinDivert on Windows, VPNService on Android, Network Extension on iOS/macOS). Document any platform constraints in the implementation or .tasks.
