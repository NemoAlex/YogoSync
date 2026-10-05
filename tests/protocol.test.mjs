import test from 'node:test';
import assert from 'node:assert/strict';
import { packet, framePackets, normalizeReply } from '../plugins/yogosync/scripts/protocol.mjs';
import { frameFor } from '../plugins/yogosync/scripts/art.mjs';
import { YogoDevice } from '../plugins/yogosync/scripts/device.mjs';
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
