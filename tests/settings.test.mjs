import {create,catalog} from './i18n-helper.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
test('settings auto-save in order and incoming snapshots preserve number editing',async()=>{
 const elements=Object.fromEntries(['language','launch-at-login','completed-seconds','settings-message','preferences-form'].map(id=>[id,{value:'8',checked:false,checkValidity(){return Number.isInteger(Number(this.value))&&Number(this.value)>=2&&Number(this.value)<=60;}}]));
 let login=false;let preferences={completedSeconds:8},listener;const writes=[];
 const document={getElementById:id=>elements[id],activeElement:null};
 const context={document,window:{YogoI18n:create('zh-CN',catalog),__TAURI__:{core:{invoke:async(name,args)=>{if(name==='get_language_preference')return 'system';if(name==='get_autostart')return login;if(name==='set_autostart'){login=args.enabled;return login;}if(name==='get_snapshot')return{preferences:{...preferences}};writes.push(args.preferences);preferences={...args.preferences};}},event:{listen:async(_,fn)=>{listener=fn;}},window:{getCurrentWindow:()=>({onCloseRequested:async()=>{}})}}}};
 vm.runInNewContext(readFileSync('desktop/settings.js','utf8'),context);
 await new Promise(r=>setImmediate(r));
 elements['launch-at-login'].checked=true;const first=elements['launch-at-login'].onchange();
 elements['completed-seconds'].value='12';const second=elements['completed-seconds'].onchange();
 await Promise.all([first,second]);assert.equal(writes.length,1);assert.equal(login,true);assert.equal(preferences.completedSeconds,12);
 document.activeElement=elements['completed-seconds'];elements['completed-seconds'].value='2';listener({payload:{preferences}});assert.equal(elements['completed-seconds'].value,'2');
 elements['completed-seconds'].value='99';await elements['completed-seconds'].onchange();assert.equal(writes.length,1);assert.equal(elements['completed-seconds'].value,12);
});

test('language selection persists, rolls back on failure, and never writes other preferences',async()=>{
 const elements=Object.fromEntries(['language','launch-at-login','completed-seconds','settings-message','preferences-form'].map(id=>[id,{value:'8',checkValidity:()=>true}]));
 let saved='en', fail=false;const writes=[];
 const context={document:{getElementById:id=>elements[id],activeElement:null},window:{YogoI18n:create('en',catalog),__TAURI__:{
  core:{invoke:async(name,args)=>{
   if(name==='get_language_preference')return saved;
   if(name==='get_snapshot')return {preferences:{completedSeconds:8}};
   if(name==='get_autostart')return false;
   if(name==='set_language'){writes.push(args.language);if(fail)throw 'disk full';saved=args.language;return;}
   throw Error('Unexpected command: '+name);
  }},event:{listen:async()=>{}},window:{getCurrentWindow:()=>({onCloseRequested:async()=>{}})}
 }}};
 vm.runInNewContext(readFileSync('desktop/settings.js','utf8'),context);
 await new Promise(setImmediate);assert.equal(elements.language.value,'en');
 for(const value of ['zh-CN','en','system']){elements.language.value=value;await elements.language.onchange();assert.equal(saved,value);assert.equal(elements.language.disabled,false);}
 fail=true;elements.language.value='zh-CN';await elements.language.onchange();
 assert.equal(saved,'system');assert.equal(elements.language.value,'system');assert.match(elements['settings-message'].textContent,/disk full/);
 assert.deepEqual(writes,['zh-CN','en','system','zh-CN']);
});
