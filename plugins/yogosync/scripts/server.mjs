import http from 'node:http';
import { mkdir, readFile, writeFile, readdir, unlink, open } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { randomBytes } from 'node:crypto';
import { dataDir } from './paths.mjs';
import { YogoDevice, backupFile, backupMatches } from './device.mjs';
import { TaskStates } from './state.mjs';
import { frameFor, states } from './art.mjs';
const port = Number(process.env.YOGOSYNC_PORT || 19775);
const webDir = fileURLToPath(new URL('../web/', import.meta.url));
const eventsDir = path.join(dataDir,'events');
await mkdir(eventsDir,{recursive:true,mode:0o700});
const token = randomBytes(24).toString('hex');
const tasks = new TaskStates();
let device, phase=0, busy=false, error='', override=null, lastFrame='', connectedModel='', lastEventAt=null, closing=false;
let backupPath = path.join(dataDir,'device-backup.json');
let enabled=false, nextConnectionCheck=0;
const lockPath = path.join(dataDir,'server.lock');
try { const lock=await open(lockPath,'wx',0o600); await lock.writeFile(String(process.pid)); await lock.close(); }
catch {
  const pid=Number(await readFile(lockPath,'utf8').catch(()=>''));
  let alive=false; try { process.kill(pid,0); alive=true; } catch {}
  if(alive) throw new Error(`YogoSync 已运行 (PID ${pid})`);
  await unlink(lockPath).catch(()=>{}); const lock=await open(lockPath,'wx',0o600); await lock.writeFile(String(process.pid)); await lock.close();
}
function current() { const s=tasks.snapshot(); return { ...s, state:override?.until>Date.now()?override.state:s.state, demo:override?.until>Date.now(), connected:!!device, connection:device?.connection || '', enabled, model:connectedModel, error, lastEventAt }; }
async function exclusive(fn) { if(busy) throw new Error('设备正在处理上一项操作，请稍后重试'); busy=true; try{return await fn();}finally{busy=false;} }
async function disconnect() {
  if(!device)return;
  const d=device;
  try { await d.restore(); await unlink(backupPath).catch(()=>{}); }
  finally { device=null;connectedModel='';lastFrame='';await d.close(); }
}
async function connect() {
  if(device)return;
  const d=await YogoDevice.open();
  backupPath=path.join(dataDir,backupFile(d.info));
  try {
    const backup=JSON.parse(await readFile(backupPath,'utf8').catch(()=>'null'));
    if(backup) {
      if(!backupMatches(backup,d.info))throw new Error('存在另一台设备的恢复备份，请先处理');
      await d.restore(backup.block); await unlink(backupPath);
    }
    await d.begin(b=>writeFile(backupPath,JSON.stringify(b),{mode:0o600}));
    device=d; connectedModel=d.info.product;error='';lastFrame='';
  } catch(e) { if(d.original)await d.restore().catch(()=>{});await d.close();throw e; }
}
async function consumeEvents() {
  const files=(await readdir(eventsDir)).filter(f=>/^\d+-[a-f0-9-]+\.json$/.test(f)).sort();
  const events=[];
  for(const f of files.slice(0,500)) {
    const p=path.join(eventsDir,f);
    try { const e=JSON.parse(await readFile(p,'utf8')); events.push(e); await unlink(p); } catch {}
  }
  for(const e of events.sort((a,b)=>a.at-b.at)) { tasks.apply(e);lastEventAt=Math.max(lastEventAt||0,e.at||0); }
}
const poll=setInterval(async()=>{
  if(busy||closing)return;
  busy=true;
  try {
    await consumeEvents();
    if(enabled && Date.now() >= nextConnectionCheck) {
      nextConnectionCheck=Date.now()+5000;
      if(device) await device.readDotBlock();
      else await connect();
    }
    // Status changes only for first hardware release: do not assume flash endurance.
    const frame=frameFor(current().state,0), key=JSON.stringify(frame);
    if(device&&key!==lastFrame) { await device.writeFrame(frame);lastFrame=key; }
    phase++;
  } catch(e) { error=e.message; if(device){await device.close().catch(()=>{});device=null;connectedModel='';} }
  finally{busy=false;}
},200);
const server=http.createServer(async(req,res)=>{
  const host=`127.0.0.1:${port}`;
  if(req.headers.host!==host){res.writeHead(403);res.end('Invalid host');return;}
  res.setHeader('Cache-Control','no-store');
  res.setHeader('X-Content-Type-Options','nosniff');
  res.setHeader('Content-Security-Policy',"default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'");
  try {
    if(req.method==='GET'&&req.url==='/api/status'){res.setHeader('Content-Type','application/json');res.end(JSON.stringify({...current(),frame:frameFor(current().state,0)}));return;}
    if(req.method==='POST'){
      if(req.headers['x-yogo-token']!==token||req.headers.origin!==`http://${host}`){res.writeHead(403);res.end('Forbidden');return;}
      if(req.url==='/api/connect')await exclusive(async()=>{enabled=true;nextConnectionCheck=Date.now()+5000;await connect();});
      else if(req.url==='/api/disconnect')await exclusive(async()=>{enabled=false;await disconnect();});
      else if(req.url?.startsWith('/api/demo/')){const state=req.url.split('/').pop();if(!states.includes(state))throw new Error('未知状态');override={state,until:Date.now()+5000};}
      else if(req.url==='/api/auto')override=null;
      else {res.writeHead(404);res.end();return;}
      res.setHeader('Content-Type','application/json');res.end(JSON.stringify(current()));return;
    }
    const files={'/':'index.html','/app.js':'app.js','/style.css':'style.css'};
    const file=files[req.url];
    if(req.method!=='GET'||!file){res.writeHead(404);res.end();return;}
    let content=await readFile(path.join(webDir,file),'utf8');
    if(file==='index.html')content=content.replace('__TOKEN__',token);
    res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html; charset=utf-8');res.end(content);
  }catch(e){error=e.message;res.writeHead(409,{'Content-Type':'application/json'});res.end(JSON.stringify({error:e.message}));}
});
server.listen(port,'127.0.0.1',()=>console.log(`YogoSync: http://127.0.0.1:${port}\nData: ${dataDir}`));
async function shutdown(){
  if(closing)return;closing=true;clearInterval(poll);
  while(busy)await new Promise(r=>setTimeout(r,100));
  try{await disconnect();}catch(e){console.error('恢复未完成：',e.message);}
  await unlink(lockPath).catch(()=>{});server.close();
}
process.on('SIGINT',shutdown);process.on('SIGTERM',shutdown);
server.on('error',async e=>{console.error(e.message);await shutdown();process.exitCode=1;});
