import HID from 'node-hid';
import { randomInt } from 'node:crypto';
import { VID, PID, USAGE_PAGE, USAGE, packet, framePackets, normalizeReply } from './protocol.mjs';
export const listDevices = async () => (await HID.devicesAsync(VID, PID)).filter(d => d.usagePage === USAGE_PAGE && d.usage === USAGE);
export class YogoDevice {
  constructor(handle, info) { this.handle = handle; this.info = info; this.sequence = randomInt(1, 65535); this.lastStart = 0; }
  nextSequence() { this.sequence = this.sequence % 65535 + 1; return this.sequence; }
  static async open() {
    const devices = await listDevices();
    if (devices.length !== 1) throw new Error(devices.length ? '请只连接一个 YOGO 75 PRO 接收器' : '未找到 YOGO 75 PRO 2.4G 接收器');
    return new YogoDevice(await HID.HIDAsync.open(devices[0].path, { nonExclusive: true }), devices[0]);
  }
  async request(command, options = {}) {
    const sequence = this.nextSequence();
    const p = packet(command, { ...options, sequence });
    await this.handle.write(Buffer.concat([Buffer.from([0]), p]));
    const deadline = Date.now() + 3000;
    while (Date.now() < deadline) {
      const reply = normalizeReply(await this.handle.read(300));
      if (reply.length < 8 || reply[0] !== 0xaa || reply[1] !== command || reply.readUInt16LE(5) !== sequence) continue;
      if (reply[2] === 255) throw new Error(`设备拒绝命令 0x${command.toString(16)}`);
      return reply.subarray(8);
    }
    throw new Error(`设备响应超时 0x${command.toString(16)}；请关闭 ATK 设置页后重试`);
  }
  async start() { if (Date.now() - this.lastStart > 3000) { await this.request(0x10); this.lastStart = Date.now(); } }
  async readDotBlock() { await this.start(); return Buffer.from(await this.request(0x14, { offset: 24, length: 24 })); }
  async writeDotBlock(block) {
    // Firmware commits the staged configuration after the final segment.
    const first = await this.request(0x14, { offset: 0, length: 24 });
    const last = await this.request(0x14, { offset: 48, length: 16 });
    await this.request(0x15, { offset: 0, length: 24, data: first.subarray(0,24) });
    await this.request(0x15, { offset: 24, length: 24, data: block });
    await this.request(0x15, { offset: 48, length: 16, data: last.subarray(0,16) });
    await new Promise(r => setTimeout(r, 250));
  }
  async begin(saveBackup) {
    const original = await this.readDotBlock();
    if (original.length !== 24) throw new Error('点阵配置长度不正确');
    if (original[6] === 6) throw new Error('当前已是自定义模式，无法备份原图案。请先在 ATK 选择一个预设灯效后重试。');
    this.original = original;
    await saveBackup?.({ vendorId: VID, productId: PID, serialNumber: this.info.serialNumber, block: [...original] });
    const changed = Buffer.from(original);
    changed[5] = 0; changed[6] = 6; // only dot on/off, mode; retain brightness and every other field
    await this.request(0x3d);
    await this.writeDotBlock(changed);
    const check = await this.readDotBlock();
    if (!check.equals(changed)) throw new Error('点阵模式写入校验失败');
  }
  async writeFrame(frame) {
    await this.start();
    for (const p of framePackets(frame, () => this.nextSequence())) await this.handle.write(Buffer.concat([Buffer.from([0]), p]));
  }
  async restore(block = this.original) {
    if (!block) return;
    await this.start();
    // Read-modify-write to preserve any unrelated settings changed after connection.
    const current = await this.readDotBlock();
    Buffer.from(block).copy(current, 5, 5, 14);
    await this.request(0x3d);
    await this.writeDotBlock(current);
    const check = await this.readDotBlock();
    if (!check.equals(current)) throw new Error('恢复原灯效校验失败');
    this.original = undefined;
  }
  async close() { await this.handle.close(); }
}
