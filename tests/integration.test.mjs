import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
test('real hook process -> event files -> service; cross-origin control rejected',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'yogo-integration-'));
 const port=24000+Math.floor(Math.random()*1000);
 const env={...process.env,YOGO_PET_HOME:dir,YOGO_PET_PORT:String(port)};
 const child=spawn(process.execPath,['plugins/yogo-pet/scripts/server.mjs'],{env,stdio:['ignore','pipe','pipe']});
 const base=`http://127.0.0.1:${port}`;let output='';child.stderr.on('data',d=>output+=d);
 try {
  await Promise.race([once(child.stdout,'data'),new Promise((_,reject)=>setTimeout(()=>reject(new Error(`Server start timeout ${output}`)),5000).unref())]);
  const send=event=>{const r=spawnSync(process.execPath,['plugins/yogo-pet/scripts/hook.mjs'],{env,input:JSON.stringify({session_id:'integration',turn_id:'turn',hook_event_name:event}),encoding:'utf8'});assert.equal(r.status,0);};
  const waitState=async expected=>{for(let i=0;i<30;i++){const s=await (await fetch(base+'/api/status')).json();if(s.state===expected)return s;await new Promise(r=>setTimeout(r,100));}throw new Error(`Missing ${expected}`);};
  send('UserPromptSubmit');assert.equal((await waitState('thinking')).activeTasks,1);
  send('PreToolUse');await waitState('working');
  send('PermissionRequest');await waitState('waiting');
  send('Stop');assert.equal((await waitState('done')).activeTasks,0);
  const page=await (await fetch(base)).text();const token=page.match(/name="yogo-token" content="([a-f0-9]+)"/)[1];
  assert.equal((await fetch(base+'/api/connect',{method:'POST',headers:{origin:'https://example.com','x-yogo-token':token}})).status,403);
  assert.equal((await fetch(base+'/api/connect',{method:'POST',headers:{origin:base}})).status,403);
  assert.equal((await fetch(base+'/api/demo/idle',{method:'POST',headers:{origin:base,'x-yogo-token':token}})).status,200);
  send('SessionEnd');
 }finally{const exit=once(child,'exit');child.kill('SIGTERM');await exit;await rm(dir,{recursive:true,force:true});}
});
