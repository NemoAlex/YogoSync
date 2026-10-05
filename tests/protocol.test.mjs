import test from 'node:test';
import assert from 'node:assert/strict';
import { packet, framePackets, normalizeReply, connectionFor, VID, PID, USB_PID, USAGE_PAGE, USAGE } from '../plugins/yogosync/scripts/protocol.mjs';
import { frameFor } from '../plugins/yogosync/scripts/art.mjs';
import { YogoDevice, openDevice, backupMatches } from '../plugins/yogosync/scripts/device.mjs';
test('ATK 32-byte frame fixture: offsets, LE request IDs, RGB tail padding',()=>{
  let n=0x1233;const f=Array.from({length:6},()=>Array.from({length:6},()=>[1,2,3]));
  const p=framePackets(f,()=>++n);
  assert.equal(p.length,5);
  assert.deepEqual([...p[0].subarray(0,8)],[170,59,0,0,24,0x34,0x12,0]);
  assert.deepEqual(p.map(b=>b.readUInt16LE(2)),[0,24,48,72,96]);
  assert.deepEqual([...p[4].subarray(8,20)],[1,2,3,1,2,3,1,2,3,1,2,3]);
  assert.ok(p[4].subarray(20).every(v=>v===0));
});
test('invalid frames never reach transport',()=>{
 assert.throws(()=>framePackets([[1]],()=>1));
 const f=frameFor('idle');f[0][0][0]=256;assert.throws(()=>framePackets(f,()=>1));
 assert.throws(()=>packet(0x15,{data:Array(25).fill(0)}));
});
test('normalizes report ID without corrupting unnumbered reports',()=>{
 assert.deepEqual(normalizeReply([0,170,20,0]),Buffer.from([170,20,0]));
 assert.deepEqual(normalizeReply([170,20,0]),Buffer.from([170,20,0]));
});
test('full configuration commit preserves unknown bytes and restores only dot fields',async()=>{
 const config=Buffer.from(Array.from({length:64},(_,i)=>i));config[30]=7;
 const original=Buffer.from(config);const writes=[];
 const d=new YogoDevice({},{});
 d.request=async(c,o={})=>{if(c===0x14)return Buffer.from(config.subarray(o.offset,o.offset+o.length));if(c===0x15){Buffer.from(o.data).copy(config,o.offset);writes.push(o.offset);}return Buffer.alloc(24);};
 await d.begin();assert.deepEqual(writes,[0,24,48]);
 assert.equal(config[30],6);assert.equal(config[29],0);
 for(let i=0;i<64;i++)if(![29,30].includes(i))assert.equal(config[i],original[i]);
 config[42]=222;await d.restore();
 assert.equal(config[42],222);assert.deepEqual(config.subarray(29,38),original.subarray(29,38));
});

const deviceInfo = (productId, path) => ({vendorId:VID,productId,usagePage:USAGE_PAGE,usage:USAGE,path,serialNumber:'same'});
function probeHandle(responds, calls, path) {
 let request;
 return {
  async write(p) { request=Buffer.from(p).subarray(1);calls.push([path,request[1]]); if(!responds) throw new Error('offline'); },
  async read() { const reply=Buffer.from(request);reply[2]=0;return reply; },
  async close() { calls.push([path,'closed']); }
 };
}
test('USB control interface is accepted, Bluetooth and keyboard inputs are excluded',()=>{
 assert.equal(connectionFor(deviceInfo(USB_PID,'usb')),'usb');
 assert.equal(connectionFor(deviceInfo(PID,'receiver')),'receiver');
 assert.equal(connectionFor({...deviceInfo(USB_PID,'keyboard'),usagePage:1,usage:6}),null);
 assert.equal(connectionFor({...deviceInfo(12292,'bluetooth'),vendorId:14000}),null);
 assert.equal(connectionFor(deviceInfo(4508,'different-model')),null);
});
test('both transports present: probe USB first without writing settings',async()=>{
 const calls=[];
 const d=await openDevice([deviceInfo(PID,'receiver'),deviceInfo(USB_PID,'usb')],async info=>probeHandle(true,calls,info.path));
 assert.equal(d.connection,'usb');assert.deepEqual(calls,[['usb',0x10],['usb',0x14]]);await d.close();
});
test('failed USB probe closes its handle and falls back to a responsive receiver',async()=>{
 const calls=[];
 const d=await openDevice([deviceInfo(PID,'receiver'),deviceInfo(USB_PID,'usb')],async info=>probeHandle(info.path==='receiver',calls,info.path));
 assert.equal(d.connection,'receiver');assert.deepEqual(calls,[['usb',0x10],['usb','closed'],['receiver',0x10],['receiver',0x14]]);await d.close();
});
test('an offline dongle cannot become a connected keyboard, and ambiguous devices are rejected',async()=>{
 const calls=[];
 await assert.rejects(openDevice([deviceInfo(PID,'receiver')],async info=>probeHandle(false,calls,info.path)),/offline/);
 assert.equal(calls.at(-1)[1],'closed');
 await assert.rejects(openDevice([deviceInfo(USB_PID,'a'),deviceInfo(USB_PID,'b')],async()=>{throw new Error('must not open');}),/多个/);
});
test('recovery requires the same transport and identity even with matching serials',()=>{
 const usb=deviceInfo(USB_PID,'usb');
 assert.ok(backupMatches(usb,usb));
 assert.equal(backupMatches(deviceInfo(PID,'receiver'),usb),false);
 assert.equal(backupMatches({...usb,serialNumber:'other'},usb),false);
});

test('USB frames use 64-byte reports and two 56-byte payload chunks',()=>{
 let sequence=4320;
 assert.equal(packet(0x10,{sequence:++sequence,reportSize:64}).toString('hex'),'aa10000000e11000'+'00'.repeat(56));
 const frame=Array.from({length:6},()=>Array.from({length:6},()=>[1,2,3]));
 const packets=framePackets(frame,()=>++sequence,64);
 assert.deepEqual(packets.map(p=>[p.length,p.readUInt16LE(2),p[4]]),[[64,0,56],[64,56,56]]);
 assert.deepEqual(Buffer.concat(packets.map(p=>p.subarray(8))).subarray(0,108),Buffer.from(frame.flat(2)));
 assert.ok(packets[1].subarray(60).every(v=>v===0));
 assert.throws(()=>packet(0x15,{length:57,reportSize:64}));
});
test('USB configuration uses 56+8 commit and retains bytes outside dot fields',async()=>{
 const config=Buffer.from(Array.from({length:64},(_,i)=>i));config[30]=2;
 const original=Buffer.from(config),writes=[];
 const d=new YogoDevice({},deviceInfo(USB_PID,'usb'));
 d.request=async(command,o={})=>{
  if(command===0x14){const reply=Buffer.alloc(56);config.subarray(o.offset,o.offset+o.length).copy(reply);return reply;}
  if(command===0x15){Buffer.from(o.data).copy(config,o.offset);writes.push([o.offset,o.length]);}
  return Buffer.alloc(56);
 };
 await d.begin();assert.deepEqual(writes,[[0,56],[56,8]]);
 assert.equal(config[30],6);assert.equal(config[29],0);
 for(let i=0;i<64;i++)if(![29,30].includes(i))assert.equal(config[i],original[i]);
 config[60]=222;await d.restore();
 assert.equal(config[60],222);assert.deepEqual(config.subarray(29,38),original.subarray(29,38));
});
