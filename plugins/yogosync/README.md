# YogoSync legacy Node prototype

This directory preserves the early Node.js implementation for protocol checks and local development. The supported desktop application and direct Codex Hooks setup are documented in the [project README](../../README.md).

Run `npm ci` followed by `npm start` in this directory, then open http://127.0.0.1:19775. Stop with Ctrl+C to restore lighting. Close other ATK or YogoSync controllers before testing.

The CLI supports `devices`, `inspect`, `demo`, and `restore`. USB wired mode (14139:4507) and the 2.4 GHz receiver (14139:4607) are detected automatically, preferring a responsive USB interface. Bluetooth matrix control is unsupported.

The prototype displays static state icons. The desktop app supports animated themes. Task events contain only event names, task identifiers, and timestamps; conversation content is not collected. Custom mode 6 restores to built-in preset 0 because its original pixels cannot be read back.

This is not an official ATK or OpenAI product.
