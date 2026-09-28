# YOGO 75 PRO receiver protocol observations

Source: ATK HUB public bundle, 3.2.27, inspected 2026-09-24.
https://bpcdn.atkgear.com/hub-v3/production/3.2.27/static/index-DBE__Npj.js

Vendor assets are not bundled or executed by this project. The implementation is independently written.

- Hardware: VID 14139, PID 4607, usage page 65376 (`FF60`), usage 97 (`61`).
- macOS HID descriptor verified: unnumbered output/input report, 32 bytes.
- node-hid writes must prepend zero report ID to the 32-byte payload.
- Header: `AA command offsetLE16 length requestLE16 00`; response data begins at byte 8.
- Response must match request ID and command; byte 2 equal to FF signals failure in vendor implementation.
- Commands used: `10` start communication; `14` read configuration; `15` write configuration; `3B` RGB frame; `3D` stop synchronized dot display.
- Dot RGB: row-major, 6 rows × 6 columns × RGB888 = 108 bytes, offsets 0/24/48/72/96, padded to 24 bytes per frame packet. Matches vendor `h0t` implementation.
- Configuration is read and committed as 24/24/16-byte segments at 0/24/48. Hardware testing established a single middle-segment write is insufficient; all segments must be committed.
- Dot fields at absolute bytes 29–37: on/off, mode, brightness, speed, colorful, color index, R/G/B. Custom mode is 6.
- Mode changes are read back and verified. Unknown config bytes and key settings are preserved, unlike rebuilding config from named fields.
- This release uses change-only static frames. Firmware persistence and sustained animation endurance have not been established.

On 2026-09-24: read-only handshake and config read succeeded on physical receiver. Three-icon demo sent successfully; original effect mode and other config bytes passed readback after restore. User observed the repeated 5-second-per-state test and confirmed all icons displayed correctly.

Codex references:
https://learn.chatgpt.com/docs/hooks
https://developers.openai.com/plugins/build/plugins
