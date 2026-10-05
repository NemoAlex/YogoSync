# YogoSync desktop architecture

The macOS app uses Tauri for windows, menus, tray access, and login startup. A single Rust worker owns HID access, task aggregation, animation playback, and recovery; UI commands wait asynchronously without blocking the window.

## Data flow

```text
Codex user Hooks → yogosync-hook → private event spool
  → TaskStates → active theme frame → DeviceService → keyboard matrix
ServiceRuntime → DesktopSnapshot → snapshot-changed → desktop windows
```

Events contain only event names, task identifiers, and timestamps. The native service has no HTTP control port. Files are written atomically with private permissions.

## Device selection and recovery

Only YOGO 75 PRO vendor 14139, USB product 4507 / receiver product 4607, usage page FF60 and usage 61 are accepted. USB uses 64-byte HID reports; the receiver uses 32-byte reports. A successful handshake and configuration read are required before selection. USB is preferred. Connectivity is checked even for static frames, and enabled services retry after disconnection.

Mode changes preserve the full 64-byte configuration and verify readback. Restoration replaces only the nine matrix fields. Recovery records are separated by transport and checked against vendor, product, and serial. Unknown custom pixels cannot be read back, so custom mode 6 recovers to built-in preset 0. See [protocol.md](protocol.md) for framing and hardware verification.

Quit waits for restoration; pending records remain available after a failed restore. Theme editor drafts participate in quit confirmation.

## Hooks and settings

The desktop app installs the helper in `$CODEX_HOME/yogosync/bin` and merges user `hooks.json`, preserving unrelated hooks and backing up valid configuration before changes. Authorization remains in Codex.

The language setting supports system default, Simplified Chinese, and English. Language changes update every window and native menu without discarding editor drafts. Cover appearance and completion duration are persisted separately from theme pixel content.

## Distribution

The supported artifact is a macOS arm64 app and DMG for macOS 12 or later. Packages are ad-hoc signed, without Apple Developer ID signing or notarization. Other platforms require separate builds and hardware verification.
