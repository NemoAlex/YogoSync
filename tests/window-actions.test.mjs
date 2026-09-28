import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
function fixture(){
 const calls=[];let onKey,release;
 const context={navigator:{platform:'MacIntel'},document:{activeElement:{blur(){}},addEventListener(_,fn){onKey=fn}},window:{__TAURI__:{core:{invoke:async(name,args)=>{calls.push([name,args]);if(name==='set_unsaved_changes')await new Promise(r=>release=r);}}}}};
 vm.runInNewContext(readFileSync(new URL('../desktop/window-actions.js',import.meta.url),'utf8'),context);
 return {calls,api:context.window.YogoWindow,key:(key,extra={})=>onKey({key,metaKey:true,preventDefault(){},...extra}),release:()=>release()};
}
test('Command-W closes the calling window; repeated or modified shortcuts are ignored',async()=>{
 const f=fixture();await f.key('w');assert.equal(f.calls[0][0],'close_current_window');
 await f.key('w',{repeat:true});await f.key('w',{shiftKey:true});await f.key('q',{metaKey:false});assert.equal(f.calls.length,1);
});
test('Command-Q waits for unsaved-state registration before requesting a safe quit',async()=>{
 const f=fixture();const registered=f.api.setDirty(true);const quitting=f.key('q');await new Promise(setImmediate);
 assert.deepEqual(f.calls.map(c=>c[0]),['set_unsaved_changes']);f.release();await registered;await quitting;
 assert.deepEqual(f.calls.map(c=>c[0]),['set_unsaved_changes','quit_app']);
 await f.api.setDirty(true);assert.equal(f.calls.length,2);
});
