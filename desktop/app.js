const { invoke } = window.__TAURI__.core;
const $ = id => document.getElementById(id);
const labels = {idle:'待机',thinking:'正在思考',working:'正在执行',waiting:'等待确认',done:'已完成',interrupted:'已中断'};
let actionError = '', previewsKey = '';
let snapshot, pending = false, lastLogs = '', themeOptionsKey = '', themePending = false, themeError = '';
for(let i=0;i<36;i++){const pixel=document.createElement('span');$('matrix').append(pixel);}
async function action(name,args){pending=true;actionError='';render();try{await invoke(name,args);}catch(e){actionError=String(e);}finally{pending=false;snapshot=await invoke('get_snapshot');render();}}
function render(){
 if(!snapshot)return;
 const s=snapshot,running=s.phase==='running',busy=pending||['starting','stopping'].includes(s.phase);
 $('service-title').textContent=({running:'已连接',stopped:'未连接',starting:'正在连接…',stopping:'正在恢复灯效…',error:'需要处理'})[s.phase]||s.phase;
 $('service-detail').textContent='YOGO 75 PRO';
 $('service-dot').className=`dot ${running?'green':s.phase==='error'?'red':busy?'amber':''}`;
 $('power').setAttribute('aria-checked',String(running));$('power').disabled=busy;
 $('power').setAttribute('aria-label',running?'停止并恢复原灯效':'启用点阵服务');
 $('error').hidden=!(s.error||actionError);$('error').textContent=s.error||actionError;
 const options=s.themeOptions||[], optionsKey=JSON.stringify(options);
 if(optionsKey!==themeOptionsKey){themeOptionsKey=optionsKey;$('theme-select').replaceChildren();for(const theme of options){const option=document.createElement('option');option.value=theme.id;option.textContent=theme.name;$('theme-select').append(option);}}
 $('theme-select').value=s.activeThemeId||'';$('theme-select').disabled=busy||themePending||!options.length;
 $('theme-error').hidden=!themeError;$('theme-error').textContent=themeError;
 $('themes').title=`当前主题：${s.themeName||'小机器人'}`;
 const label=labels[s.state];$('pet-state').textContent=label;
 $('matrix').setAttribute('aria-label',`6 × 6 点阵预览：${label}`);
 window.YogoGlow.render($('matrix'),s.frame);
 const key=JSON.stringify(s.themePreviews||{});if(key!==previewsKey){previewsKey=key;for(const button of $('state-previews').children){const frame=s.themePreviews?.[button.dataset.demo];if(frame)window.YogoGlow.render(button.querySelector('.glow-preview'),frame);}}
 $('mode-badge').textContent=s.demo?'预览中 · 5 秒':'自动跟随';$('mode-badge').className=`badge ${s.demo?'blue':''}`;
 $('automatic').hidden=!s.demo;
 document.querySelectorAll('[data-demo]').forEach(button=>{button.disabled=busy;button.classList.toggle('selected',button.dataset.demo===s.state);button.setAttribute('aria-pressed',String(button.dataset.demo===s.state));});
 $('log-count').textContent=s.logs.length;
 const logKey=JSON.stringify(s.logs);if(logKey!==lastLogs){lastLogs=logKey;$('logs').replaceChildren();for(const log of [...s.logs].reverse()){const row=document.createElement('div');row.className=`log-entry ${log.level}`;const time=document.createElement('span');time.className='log-time';time.textContent=new Date(log.at).toLocaleTimeString('zh-CN',{hour12:false});const message=document.createElement('span');message.textContent=log.message;row.append(time,message);$('logs').append(row);}if(!s.logs.length){const e=document.createElement('p');e.className='empty';e.textContent='暂时没有日志';$('logs').append(e);}}
}
$('power').onclick=()=>action(snapshot?.phase==='running'?'stop_service':'start_service');
$('settings').onclick=()=>invoke('open_settings');$('automatic').onclick=()=>action('automatic');$('clear-logs').onclick=()=>action('clear_logs');
for(const [state,label] of Object.entries({idle:'待机',thinking:'思考',working:'执行',waiting:'等待确认',done:'完成',interrupted:'中断'})){
 const button=document.createElement('button');button.dataset.demo=state;button.title=`预览${label} · 5 秒`;
 const matrix=document.createElement('span');matrix.className='glow-preview';matrix.setAttribute('aria-hidden','true');for(let i=0;i<36;i++)matrix.append(document.createElement('span'));
 const caption=document.createElement('span');caption.textContent=label;button.append(matrix,caption);button.onclick=()=>action('preview_state',{state});$('state-previews').append(button);
}
document.querySelectorAll('[data-tab]').forEach(button=>button.onclick=()=>{document.querySelectorAll('[data-tab]').forEach(b=>{b.classList.toggle('active',b===button);b.setAttribute('aria-pressed',String(b===button));});$('status-panel').hidden=button.dataset.tab!=='status';$('logs-panel').hidden=button.dataset.tab!=='logs';});
(async()=>{await window.__TAURI__.event.listen('snapshot-changed',event=>{snapshot=event.payload;render();});snapshot=await invoke('get_snapshot');render();})();

$('themes').onclick=()=>invoke('open_themes');

$('theme-select').onchange=async()=>{
 const id=$('theme-select').value;if(!id||id===snapshot?.activeThemeId)return;
 themePending=true;themeError='';render();
 try{await invoke('activate_theme',{id});snapshot=await invoke('get_snapshot');}
 catch(e){themeError=String(e);}
 finally{themePending=false;render();}
};
