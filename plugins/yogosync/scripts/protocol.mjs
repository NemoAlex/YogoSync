// Independently implemented from ATK HUB 3.2.27's public device protocol.
export const VID = 14139;
export const PID = 4607;
export const USB_PID = 4507; // Official YOGO 75 PRO wired interface; Bluetooth is excluded.
export const USAGE_PAGE = 65376;
export const USAGE = 97;
export function packet(command, { offset = 0, length = 0, data = [], sequence = 1, reportSize = 32 } = {}) {
  if (![32,64].includes(reportSize) || !Number.isInteger(offset) || offset < 0 || offset > 65535 || length < 0 || length > reportSize - 8 || data.length > reportSize - 8) throw new Error('Invalid packet bounds');
  const out = Buffer.alloc(reportSize);
  out[0] = 0xaa; out[1] = command;
  out.writeUInt16LE(offset, 2); out[4] = length;
  out.writeUInt16LE(sequence, 5); Buffer.from(data).copy(out, 8);
  return out;
}
export function framePackets(frame, nextSequence, reportSize = 32) {
  if (!Array.isArray(frame) || frame.length !== 6 || frame.some(row => !Array.isArray(row) || row.length !== 6 || row.some(rgb => !Array.isArray(rgb) || rgb.length !== 3 || rgb.some(c => !Number.isInteger(c) || c < 0 || c > 255)))) throw new Error('Expected 6×6 RGB frame');
  const rgb = frame.flat(2);
  if (![32,64].includes(reportSize)) throw new Error('Invalid report size');
  const size = reportSize - 8;
  return Array.from({length:Math.ceil(rgb.length/size)},(_,i)=>i*size).map(offset => packet(0x3b, { offset, length: size, data: rgb.slice(offset, offset + size), sequence: nextSequence(), reportSize }));
}
export function normalizeReply(data) {
  const b = Buffer.from(data);
  return b[0] !== 0xaa && b[1] === 0xaa ? b.subarray(1) : b;
}

export function connectionFor(info) {
  if (info.vendorId !== VID || info.usagePage !== USAGE_PAGE || info.usage !== USAGE) return null;
  return info.productId === USB_PID ? 'usb' : info.productId === PID ? 'receiver' : null;
}
