const $=s=>document.querySelector(s);
const token=$('meta[name="yogo-token"]').content;
const labels={idle:'待机',thinking:'思考中',working:'执行中',waiting:'等你确认',done:'本轮回复完成',interrupted:'已中断'};
const pixels=Array.from({length:36},()=>{const p=document.createElement('div');p.className='pixel';$('#matrix').append(p);return p});
let pending=false;
async function action(endpoint){if(pending)return;pending=true;try{const r=await fetch(endpoint,{method:'POST',headers:{'x-yogo-token':token}});const v=await r.json();if(!r.ok)throw new Error(v.error);await refresh();}catch(e){$('#error').textContent=e.message;}finally{pending=false;}}
$('#connect').onclick=()=>action('/api/connect');$('#disconnect').onclick=()=>action('/api/disconnect');
document.querySelectorAll('[data-state]').forEach(b=>b.onclick=()=>action('/api/demo/'+b.dataset.state));
async function refresh(){try{
 const r=await fetch('/api/status');const s=await r.json();
 s.frame.flat().forEach((rgb,i)=>{const lit=rgb.some(Boolean),color=`rgb(${rgb.join(',')})`;pixels[i].style.background=lit?color:'#142019';pixels[i].style.boxShadow=lit?`0 0 12px rgba(${rgb.join(',')},0.33)`:'none';});
 $('#state').textContent=labels[s.state];$('#mode').textContent=s.demo?'体验模式':'任务同步';
 $('#connection').textContent=s.connected?`已连接 · ${s.model} · ${s.connection==='usb'?'USB 有线':'2.4 GHz 接收器'}`:'支持 YOGO 75 PRO · USB / 2.4G';
 $('#connect').disabled=s.connected;$('#disconnect').disabled=!s.enabled;$('#error').textContent=s.error;
 $('#events').textContent=s.lastEventAt?`已收到 Codex 事件 · ${new Date(s.lastEventAt).toLocaleTimeString()} · ${s.activeTasks} 个活动任务`:'等待 Codex 事件 · 安装插件并信任 Hooks 后自动同步';
 document.querySelectorAll('[data-state]').forEach(b=>b.classList.toggle('active',b.dataset.state===s.state));
}catch{$('#error').textContent='本地服务已断开，请重新启动 YogoSync';}}
await refresh();setInterval(refresh,700);
