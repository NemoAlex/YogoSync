import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readdir, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
test('hook emits minimum state metadata and never stores prompt or tool arguments',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'yogo-hook-'));
 try {
  const r=spawnSync(process.execPath,['plugins/yogo-pet/scripts/hook.mjs'],{input:JSON.stringify({hook_event_name:'UserPromptSubmit',session_id:'a',turn_id:'b',prompt:'PRIVATE',tool_input:{secret:'PRIVATE'}}),env:{...process.env,YOGO_PET_HOME:dir},encoding:'utf8'});
  assert.equal(r.status,0);assert.deepEqual(JSON.parse(r.stdout),{});
  const files=await readdir(path.join(dir,'events'));assert.equal(files.length,1);
  const raw=await readFile(path.join(dir,'events',files[0]),'utf8');assert.ok(!raw.includes('PRIVATE'));
  assert.equal(JSON.parse(raw).event,'UserPromptSubmit');
 }finally{await rm(dir,{recursive:true,force:true});}
});
