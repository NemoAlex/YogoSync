import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';

async function fixture(initialState) {
  const nodes=new Map(), calls=[];
  let state=initialState, snapshotListener, focusListener, failure=false;
  const node=id=>{if(!nodes.has(id))nodes.set(id,{setAttribute(key,value){this[key]=value},parentElement:{scrollTop:0},focus(){},showModal(){this.open=true},close(){this.open=false}});return nodes.get(id)};
  const context={document:{getElementById:node,addEventListener(){}},window:{addEventListener(name,fn){if(name==='focus')focusListener=fn},__TAURI__:{core:{async invoke(name){calls.push(name);if(name==='get_codex_connection'){if(failure)throw Error('offline');return{state,detail:'test'}}if(name==='get_snapshot')return{tasks:{lastEventAt:null}}}},event:{listen(_,fn){snapshotListener=fn;return Promise.resolve()}}}},setInterval(){},Date};
  vm.runInNewContext(readFileSync(new URL('../desktop/codex-connection.js',import.meta.url),'utf8'),context);
  await new Promise(setImmediate);
  return {node,calls,async change(next){state=next;await focusListener()},event(){snapshotListener({payload:{tasks:{lastEventAt:Date.now()}}})},async fail(){failure=true;await focusListener()}};
}

test('onboarding actions route to installation, plugin enablement and trust review',async()=>{
  const f=await fixture('missing');
  for(const [state,label,command] of [['missing','安装插件','install_codex_plugin'],['plugin_disabled','前往启用','open_codex_plugin'],['unauthorized','前往授权',null],['modified','前往授权',null],['disabled','前往启用',null]]){
    await f.change(state);assert.equal(f.node('codex-authorize').hidden,false);assert.equal(f.node('codex-authorize').textContent,label);
    const before=f.calls.length;await f.node('codex-authorize').onclick();if(command){assert.equal(f.calls.at(-1),command)}else{assert.equal(f.calls.length,before);assert.equal(f.node('authorization-guide').open,true);f.node('guide-close').onclick();assert.equal(f.node('authorization-guide').open,false)};assert.notEqual(f.node('codex-status').textContent,'已连接');
  }
});
test('authorization alone is not connection; events do not override revoked trust or failed detection',async()=>{
  const f=await fixture('authorized');assert.equal(f.node('codex-status').textContent,'已授权 · 等待事件');
  f.event();assert.equal(f.node('codex-status').textContent,'已连接');assert.equal(f.node('codex-detail').hidden,true);assert.equal(f.node('codex-detail').textContent,'');
  await f.change('modified');assert.equal(f.node('codex-status').textContent,'需要重新授权');
  await f.fail();assert.equal(f.node('codex-status').textContent,'检测失败');assert.equal(f.node('codex-authorize').hidden,true);
});
