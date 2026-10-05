(() => {
const {t, messageText, themeName} = window.YogoI18n;
const {invoke}=window.__TAURI__.core;
const $=id=>document.getElementById(id);
let snapshot, pending=0, revision=0, queue=Promise.resolve();
function draft(){return{completedSeconds:Number($('completed-seconds').value)}}
function showError(text){$('settings-message').textContent=text;$('settings-message').className='message error';$('settings-message').hidden=!text;}
function render(s,force=false){snapshot=s;if(!pending&&(force||document.activeElement!==$('completed-seconds'))){$('completed-seconds').value=s.preferences.completedSeconds;}}
function save(){
 if(!snapshot)return queue;
 const preferences=draft();
 if(!$('completed-seconds').checkValidity()){
  showError(t('完成图标停留时间应为 2–60 的整数秒'));
  render(snapshot,true);
  return queue;
 }
 const current=++revision;pending++;showError('');
 queue=queue.then(async()=>{
  try{await invoke('save_preferences',{preferences});snapshot=await invoke('get_snapshot');}
  catch(e){if(current===revision)showError(t('保存失败：')+messageText(e));}
  finally{pending--;if(!pending)render(snapshot,true);}
 });
 return queue;
}
let savedLanguage='system';
async function refreshLanguage() {
 try { savedLanguage=await invoke('get_language_preference');$('language').value=savedLanguage;$('language').disabled=false; }
 catch(e) { showError(t('读取设置失败：')+messageText(e)); }
}
$('language').onchange=async()=>{
 const control=$('language'), language=control.value;control.disabled=true;showError('');
 pending++;
 queue=queue.then(async()=>{
  try { await invoke('set_language',{language});savedLanguage=language; }
  catch(e) { showError(t('保存失败：')+messageText(e)); }
  finally { pending--;control.value=savedLanguage;control.disabled=false; }
 });
 await queue;
};
refreshLanguage();
let loginEnabled=false;
$('launch-at-login').onchange=async()=>{
 const control=$('launch-at-login');control.disabled=true;showError('');
 try{loginEnabled=await invoke('set_autostart',{enabled:control.checked});}
 catch(e){showError(t('自动启动设置失败：')+messageText(e));}
 finally{control.checked=loginEnabled;control.disabled=false;}
};
async function refreshLogin(){
 try{loginEnabled=await invoke('get_autostart');$('launch-at-login').checked=loginEnabled;$('launch-at-login').disabled=false;}
 catch(e){showError(t('读取自动启动设置失败：')+messageText(e));}
}
refreshLogin();
$('completed-seconds').onchange=save;
$('preferences-form').onsubmit=event=>{event.preventDefault();save();};
(async()=>{
 try{
  await window.__TAURI__.event.listen('snapshot-changed',e=>render(e.payload));
  render(await invoke('get_snapshot'));
  $('completed-seconds').disabled=false;
 }catch(e){showError(t('读取设置失败：')+messageText(e));}
})();
(async()=>{
 const win=window.__TAURI__.window.getCurrentWindow();let closing=false;
 await win.onCloseRequested(async e=>{
  document.activeElement?.blur();
  if(pending){e.preventDefault();if(closing)return;closing=true;await queue;await win.destroy();}
 });
})();

})();
