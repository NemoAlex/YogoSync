import HID from 'node-hid';
import { randomInt } from 'node:crypto';
import { connectionFor, packet, framePackets, normalizeReply } from './protocol.mjs';
export const listDevices = async () => (await HID.devicesAsync()).filter(d => connectionFor(d));
export const backupFile = info => connectionFor(info) === 'usb' ? 'device-backup-usb.json' : 'device-backup.json';
export const backupMatches = (backup, info) => backup.vendorId === info.vendorId && backup.productId === info.productId && backup.serialNumber === info.serialNumber;
export async function openDevice(devices, openHandle) {
  for (const connection of ['usb', 'receiver']) {
    if (devices.filter(d => connectionFor(d) === connection).length > 1) throw new Error('检测到多个同类 YOGO 75 PRO 设备，请只保留一台键盘');
  }
  let error;
  for (const info of [...devices].filter(d => connectionFor(d)).sort((a,b) => Number(connectionFor(a) === 'receiver') - Number(connectionFor(b) === 'receiver'))) {
    let device;
    try {
      device = new YogoDevice(await openHandle(info), info);
      const block = await device.readDotBlock();
      if (block.length !== 24) throw new Error('点阵配置长度不正确');
      return device;
    } catch (e) { error = e; if (device) await device.close().catch(()=>{}); }
  }
  throw error || new Error('未找到 YOGO 75 PRO，请连接 USB 数据线并切到有线模式，或连接 2.4G 接收器');
}
export class YogoDevice {
  constructor(handle, info) { this.handle = handle; this.info = info; this.sequence = randomInt(1, 65535); this.lastStart = 0; }
  nextSequence() { this.sequence = this.sequence % 65535 + 1; return this.sequence; }
  static async open() {
    return openDevice(await listDevices(), info => HID.HIDAsync.open(info.path, { nonExclusive: true }));
  }
  get connection() { return connectionFor(this.info); }

  get reportSize() { return this.connection === 'usb' ? 64 : 32; }
  async request(command, options = {}) {
    const sequence = this.nextSequence();
    const p = packet(command, { ...options, sequence, reportSize: this.reportSize });
    await this.handle.write(Buffer.concat([Buffer.from([0]), p]));
    const deadline = Date.now() + 3000;
    while (Date.now() < deadline) {
      const reply = normalizeReply(await this.handle.read(300));
      if (reply.length < 8 || reply[0] !== 0xaa || reply[1] !== command || reply.readUInt16LE(5) !== sequence) continue;
      if (reply[2] === 255) throw new Error(`设备拒绝命令 0x${command.toString(16)}`);
      return reply.subarray(8, this.reportSize);
    }
    throw new Error(`设备响应超时 0x${command.toString(16)}；请关闭 ATK 设置页后重试`);
  }
  async start() { if (Date.now() - this.lastStart > 3000) { await this.request(0x10); this.lastStart = Date.now(); } }
  async readDotBlock() {
    await this.start();
    if (this.connection === 'usb') return Buffer.from((await this.request(0x14, {offset:0,length:56})).subarray(24,48));
    return Buffer.from(await this.request(0x14, { offset: 24, length: 24 }));
  }
  async writeDotBlock(block) {
    if (this.connection === 'usb') {
      const first = await this.request(0x14, {offset:0,length:56});
      const last = await this.request(0x14, {offset:56,length:8});
      if(first.length < 56 || last.length < 8) throw new Error('设备配置长度不正确');
      Buffer.from(block).copy(first,24);
      await this.request(0x15, {offset:0,length:56,data:first.subarray(0,56)});
      await this.request(0x15, {offset:56,length:8,data:last.subarray(0,8)});
      await new Promise(r=>setTimeout(r,250));
      return;
    }
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
    const recovery = Buffer.from(original);
    if (recovery[6] === 6) {
      recovery[5] = 0; recovery[6] = 0;
      if (!recovery[7]) recovery[7] = 50;
      if (recovery.subarray(11,14).every(v=>v===0)) recovery.fill(255,11,14);
    }
    this.original = recovery;
    await saveBackup?.({ vendorId: this.info.vendorId, productId: this.info.productId, serialNumber: this.info.serialNumber, block: [...recovery] });
    const changed = Buffer.from(original);
    changed[5] = 0; changed[6] = 6; // only dot on/off, mode; retain brightness and every other field
    await this.request(0x3d);
    await this.writeDotBlock(changed);
    const check = await this.readDotBlock();
    if (!check.equals(changed)) throw new Error('点阵模式写入校验失败');
  }
  async writeFrame(frame) {
    await this.start();
    for (const p of framePackets(frame, () => this.nextSequence(), this.reportSize)) await this.handle.write(Buffer.concat([Buffer.from([0]), p]));
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
