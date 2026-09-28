const {invoke}=window.__TAURI__.core;
const $=id=>document.getElementById(id);let snapshot,initialized=false,saving=false;
function draft(){return{autoConnect:$('auto-connect').checked,completedSeconds:Number($('completed-seconds').value)}}
function dirty(){return snapshot&&JSON.stringify(draft())!==JSON.stringify(snapshot.preferences)}
function updateDirty(){window.YogoWindow.setDirty(Boolean(dirty()));$('save').disabled=saving||!dirty();$('dirty-label').textContent=saving?'正在保存…':dirty()?'有未保存的修改':'设置已保存';}
function message(text,error=false){$('settings-message').textContent=text;$('settings-message').className=`message ${error?'error':''}`;$('settings-message').hidden=false;}
function render(s){const wasDirty=dirty();snapshot=s;if(!initialized||!wasDirty){$('auto-connect').checked=s.preferences.autoConnect;$('completed-seconds').value=s.preferences.completedSeconds;initialized=true;}updateDirty();}
$('auto-connect').onchange=updateDirty;$('completed-seconds').oninput=updateDirty;
$('preferences-form').onsubmit=async event=>{event.preventDefault();const preferences=draft();saving=true;updateDirty();try{await invoke('save_preferences',{preferences});render(await invoke('get_snapshot'));message('设置已保存');}catch(e){message(String(e),true)}finally{saving=false;updateDirty()}};
(async()=>{await window.__TAURI__.event.listen('snapshot-changed',e=>render(e.payload));render(await invoke('get_snapshot'));})();

(async()=>{const win=window.__TAURI__.window.getCurrentWindow();let closing=false;await win.onCloseRequested(async e=>{if(dirty()){e.preventDefault();if(closing)return;closing=true;try{if(await invoke('confirm_discard_changes'))await win.destroy();}finally{closing=false;}}});})();
