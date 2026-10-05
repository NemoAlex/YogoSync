# YOGO 75 PRO HID protocol observations

Source: ATK HUB public bundle, 3.2.27, inspected 2026-09-24.
https://bpcdn.atkgear.com/hub-v3/production/3.2.27/static/index-DBE__Npj.js

Vendor assets are not bundled or executed by this project. The implementation is independently written.

- Hardware: VID 14139, USB PID 4507 / receiver PID 4607, usage page 65376 (`FF60`), usage 97 (`61`).
- macOS HID descriptors verified: unnumbered output/input reports, 64 bytes for USB and 32 bytes for receiver.
- HID writes prepend zero report ID: 65 bytes for USB, 33 bytes for receiver.
- Header: `AA command offsetLE16 length requestLE16 00`; response data begins at byte 8.
- Response must match request ID and command; byte 2 equal to FF signals failure in vendor implementation.
- Commands used: `10` start communication; `14` read configuration; `15` write configuration; `3B` RGB frame; `3D` stop synchronized dot display.
- Dot RGB: row-major, 6 rows × 6 columns × RGB888 = 108 bytes. USB offsets 0/56, padded to 56 bytes per packet; receiver offsets 0/24/48/72/96, padded to 24 bytes. Matches vendor dot-matrix implementation.
- Configuration is preserved across a full 64-byte commit: USB uses 56/8-byte segments at 0/56; receiver uses 24/24/16-byte segments at 0/24/48. A middle-segment write alone is insufficient.
- Dot fields at absolute bytes 29–37: on/off, mode, brightness, speed, colorful, color index, R/G/B. Custom mode is 6.
- Mode changes are read back and verified. Unknown config bytes and key settings are preserved, unlike rebuilding config from named fields.
- This release uses change-only static frames. Firmware persistence and sustained animation endurance have not been established.

On 2026-09-24: read-only handshake and config read succeeded on physical receiver. Three-icon demo sent successfully; original effect mode and other config bytes passed readback after restore. User observed the repeated 5-second-per-state test and confirmed all icons displayed correctly.

Codex references:
https://learn.chatgpt.com/docs/hooks
https://developers.openai.com/plugins/build/plugins

Custom mode 6 is accepted without pixel backup. Its recovery record stores built-in mode 0 (the first ATK dot preset, star), retaining existing brightness/color unless zero would leave it dark. This is a dot-only fallback, not a factory reset. Recovery is still committed and read back before the record is removed.

## USB and automatic transport selection (2026-10-05)

The same official 3.2.27 bundle registers YOGO 75 PRO USB as VID 14139,
PID 4507, usage page FF60, usage 61, alongside receiver PID 4607 with
shared Evision controllers and dot commands. USB uses 64-byte reports (56 data bytes),
whereas the receiver uses 32-byte reports (24 data bytes). Their headers and configuration fields agree.

Selection probes USB first, then the receiver: 0x10 and a 24-byte 0x14 read
must succeed before a device is considered connected. A plugged-in but offline
dongle is not sufficient. Multiple candidates of the same transport are rejected.
The service checks connectivity every five seconds even for static frames, and
retries after disconnects until the user stops it. The desktop shows the active transport.

USB recovery uses device-backup-usb.json; receiver recovery retains the existing
device-backup.json. Backups match vendor, product, and serial, preventing a
transport change from restoring a different device's configuration. A disconnected
transport's recovery record is retained for the next connection of that transport.

The observed 14000:12292 ATK Yogo75 Pro-1 is Bluetooth Low Energy, not USB.
It has no FF60/61 control collection and is deliberately excluded.
USB hardware verification succeeded with both interfaces connected: automatic USB
selection, handshake, configuration reads, custom-mode write/readback, three-icon
display transmission, and restoration with a full 64-byte config comparison. The native
Rust transport also passed its opt-in hardware display/restore test. The packaged
desktop app displayed Connected / USB wired with both devices present, received
Codex Working state, and passed disconnect/restore followed by reconnect.
Physical USB-to-receiver mode switching has not yet been retested; fallback selection
is covered by simulated transport tests.
