// No model instructions, no transcript reads, no network, no HID access.
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { dataDir } from './paths.mjs';
const supported = new Set(['SessionStart','UserPromptSubmit','PreToolUse','PostToolUse','PermissionRequest','Stop','Interrupt','SessionEnd']);
try {
  let raw = '';
  for await (const chunk of process.stdin) { raw += chunk; if (raw.length > 2_000_000) throw new Error('Input too large'); }
  const input = JSON.parse(raw);
  if (supported.has(input.hook_event_name) && typeof input.session_id === 'string') {
    const folder = path.join(dataDir, 'events');
    await mkdir(folder, { recursive:true, mode:0o700 });
    const event = { event: input.hook_event_name, session: input.session_id.slice(0,160), turn: typeof input.turn_id === 'string' ? input.turn_id.slice(0,160) : '', call: typeof input.tool_use_id === 'string' ? input.tool_use_id.slice(0,160) : '', at: Date.now() };
    // One immutable file per event avoids concurrent hooks overwriting one another.
    await writeFile(path.join(folder, `${event.at}-${randomUUID()}.json`), JSON.stringify(event), { mode:0o600, flag:'wx' });
  }
} catch { /* A disconnected pet must never prevent Codex from working. */ }
process.stdout.write('{}\n');
