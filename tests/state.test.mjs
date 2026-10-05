import test from 'node:test';
import assert from 'node:assert/strict';
import { TaskStates } from '../plugins/yogosync/scripts/state.mjs';
test('one completed task cannot hide another running task',()=>{
 const s=new TaskStates();let at=Date.now();
 const send=(session,event,turn='1')=>s.apply({session,event,turn,at:at++});
 send('a','UserPromptSubmit');send('b','UserPromptSubmit');send('a','Stop');
 assert.equal(s.snapshot(at).state,'thinking');
 send('b','PermissionRequest');assert.equal(s.snapshot(at).state,'waiting');
 send('b','Stop');assert.equal(s.snapshot(at).state,'done');
 assert.equal(s.snapshot(at+9000).state,'idle');
});
test('old-turn finish and out-of-order events cannot overwrite newer work',()=>{
 const s=new TaskStates();const at=Date.now();
 s.apply({session:'a',event:'UserPromptSubmit',turn:'new',at});
 s.apply({session:'a',event:'Stop',turn:'old',at:at+1});
 s.apply({session:'a',event:'Stop',turn:'new',at:at-1});
 assert.equal(s.snapshot(at).state,'thinking');
 s.apply({session:'a',event:'Interrupt',turn:'new',at:at+2});assert.equal(s.snapshot(at+3).state,'interrupted');
});
