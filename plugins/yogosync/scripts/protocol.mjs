// Independently implemented from ATK HUB 3.2.27's public device protocol.
export const VID = 14139;
export const PID = 4607; // YOGO 75 PRO 2.4 GHz receiver; other models deliberately excluded.
export const USAGE_PAGE = 65376;
export const USAGE = 97;
export function packet(command, { offset = 0, length = 0, data = [], sequence = 1 } = {}) {
  if (!Number.isInteger(offset) || offset < 0 || offset > 65535 || length < 0 || length > 24 || data.length > 24) throw new Error('Invalid packet bounds');
  const out = Buffer.alloc(32);
  out[0] = 0xaa; out[1] = command;
  out.writeUInt16LE(offset, 2); out[4] = length;
  out.writeUInt16LE(sequence, 5); Buffer.from(data).copy(out, 8);
  return out;
}
export function framePackets(frame, nextSequence) {
  if (!Array.isArray(frame) || frame.length !== 6 || frame.some(row => !Array.isArray(row) || row.length !== 6 || row.some(rgb => !Array.isArray(rgb) || rgb.length !== 3 || rgb.some(c => !Number.isInteger(c) || c < 0 || c > 255)))) throw new Error('Expected 6×6 RGB frame');
  const rgb = frame.flat(2);
  return [0, 24, 48, 72, 96].map(offset => packet(0x3b, { offset, length: 24, data: rgb.slice(offset, offset + 24), sequence: nextSequence() }));
}
export function normalizeReply(data) {
  const b = Buffer.from(data);
  return b[0] !== 0xaa && b[1] === 0xaa ? b.subarray(1) : b;
}
