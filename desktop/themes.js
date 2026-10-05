(() => {
const {t, messageText, themeName} = window.YogoI18n;
const {invoke}=window.__TAURI__.core;
const $=id=>document.getElementById(id);
const names={idle:'待机',thinking:'思考',working:'执行',waiting:'等待确认',done:'完成',interrupted:'中断'};
const clone=v=>structuredClone(v),blank=()=>Array.from({length:6},()=>Array.from({length:6},()=>[0,0,0]));
let library,draft,baseline,state='idle',index=0,eraser=false,painting=false,playing=false,started=0,busy=false,undo=[],redo=[];
const frames=()=>draft.states[state],current=()=>frames()[index],dirty=()=>JSON.stringify(draft)!==baseline;
const rgb=p=>`rgb(${p.join(',')})`;
function message(text,error=false){$('theme-message').textContent=text;$('theme-message').className=error?'error':'';}
function checkpoint(){undo.push(JSON.stringify(draft));if(undo.length>80)undo.shift();redo=[];}
function mark(){ window.YogoWindow.setDirty(dirty()); $('dirty-status').textContent=dirty()?t('未保存'):'';$('undo').disabled=!undo.length;$('redo').disabled=!redo.length;$('apply-theme').disabled=busy||dirty()||!library.themes.some(t=>t.id===draft.id);$('save-theme').disabled=busy||!dirty(); }
function stop(){playing=false;$('play-preview').textContent=t('播放循环');}
function preview(pixels){window.YogoGlow.render($('glow-preview'),pixels);}
function paintCanvas(){[...$('pixel-editor').children].forEach((p,i)=>{const c=current().pixels.flat()[i];p.style.background=c.some(Boolean)?rgb(c):'#293142';});if(!playing)preview(current().pixels);}
function thumbnails(){const root=$('frame-list');root.replaceChildren();frames().forEach((frame,i)=>{const b=document.createElement('button');b.className='frame-card';b.setAttribute('aria-pressed',String(index===i));b.setAttribute('aria-label',frames().length===1?t('第 1 帧，静态图'):t('第 {0} 帧，{1} 毫秒',i+1,frame.durationMs));const grid=document.createElement('span');grid.className='mini-matrix';frame.pixels.flat().forEach(c=>{const dot=document.createElement('i');dot.style.background=c.some(Boolean)?rgb(c):'#293142';grid.append(dot)});const label=document.createElement('span');label.textContent=frames().length===1?t('第 1 帧'):`${i+1} · ${frame.durationMs}ms`;b.append(grid,label);b.onclick=()=>{stop();index=i;render()};root.append(b)});}
function render(){const single=frames().length===1;if(single)stop();$('frame-options').hidden=single;$('frame-duration').disabled=single;$('play-preview').hidden=single;index=Math.min(index,frames().length-1);$('theme-name').value=themeName(draft.id,draft.name);$('state-title').textContent=t(names[state]);$('frame-duration').value=current().durationMs;$('frame-count').textContent=`${frames().length}/60`;$('loop-duration').textContent=single?'':t('循环 {0} 秒',(frames().reduce((s,f)=>s+f.durationMs,0)/1000).toFixed(1));$('delete-frame').disabled=frames().length===1;$('add-frame').disabled=$('copy-frame').disabled=frames().length>=60;$('move-left').disabled=index===0;$('move-right').disabled=index===frames().length-1;for(const b of $('state-list').children){b.setAttribute('aria-pressed',String(b.dataset.state===state));b.querySelector('span').textContent=t(draft.states[b.dataset.state].length===1?'1 帧':'{0} 帧',draft.states[b.dataset.state].length);}paintCanvas();thumbnails();mark();if(!playing)$('preview-time').textContent=single?t('静态图'):t('第 {0} 帧 · {1} ms',index+1,current().durationMs);}
function choose(theme){for(const option of [...$('theme-list').options]){if(!library.themes.some(t=>t.id===option.value))option.remove();}stop();draft=clone(theme);baseline=JSON.stringify(draft);index=0;undo=[];redo=[];if(![...$('theme-list').options].some(o=>o.value===theme.id)){const o=document.createElement('option');o.value=theme.id;o.textContent=theme.name+t(' · 未保存');$('theme-list').append(o);}$('theme-list').value=theme.id;render();}
async function reload(){library=await invoke('get_themes');$('theme-list').replaceChildren();for(const theme of library.themes){const o=document.createElement('option');o.value=theme.id;o.textContent=themeName(theme.id,theme.name)+(theme.id==='builtin'?t(' · 内置'):'');$('theme-list').append(o);}$('active-theme').textContent=t('当前应用：{0}',themeName(library.activeId,library.themes.find(t=>t.id===library.activeId)?.name||'—'));}
async function discard(){if(!dirty())return true;return new Promise(resolve=>{const d=$('discard-dialog');$('keep-editing').onclick=()=>{d.close();resolve(false)};$('discard-changes').onclick=()=>{d.close();resolve(true)};d.oncancel=e=>{e.preventDefault();d.close();resolve(false)};d.showModal()});}
async function task(fn){if(busy)return;busy=true;const controls=[...document.querySelectorAll('main button,main input,main select')];const disabled=controls.map(c=>c.disabled);controls.forEach(c=>c.disabled=true);try{await fn()}catch(e){message(messageText(e),true)}finally{busy=false;controls.forEach((c,i)=>c.disabled=disabled[i]);if(draft)render()}}
for(const [key,label] of Object.entries(names)){const b=document.createElement('button');b.dataset.state=key;b.append(document.createTextNode(t(label)),document.createElement('span'));b.onclick=()=>{stop();state=key;index=0;render()};$('state-list').append(b);}
for(let i=0;i<36;i++){const b=document.createElement('button');b.setAttribute('aria-label',t('第 {0} 行，第 {1} 列',Math.floor(i/6)+1,i%6+1));b.dataset.index=i;b.onpointerdown=e=>{if(busy||e.button!==0)return;e.preventDefault();stop();checkpoint();painting=true;paint(i)};b.onkeydown=e=>{if(e.key===' '||e.key==='Enter'){e.preventDefault();stop();checkpoint();paint(i);thumbnails();mark()}};$('pixel-editor').append(b);$('glow-preview').append(document.createElement('span'));}
function paint(i){const hex=$('paint-color').value;current().pixels[Math.floor(i/6)][i%6]=eraser?[0,0,0]:[1,3,5].map(n=>parseInt(hex.slice(n,n+2),16));paintCanvas();mark();}
$('pixel-editor').onpointermove=e=>{if(!painting)return;const b=document.elementFromPoint(e.clientX,e.clientY);if(b?.parentElement===$('pixel-editor'))paint(Number(b.dataset.index))};
window.addEventListener('pointerup',()=>{if(painting){painting=false;thumbnails()}});window.addEventListener('blur',()=>{painting=false});
for(const color of ['#005a78','#0f782d','#784b00','#7d1214']){const b=document.createElement('button');b.dataset.color=color;b.style.background=color;b.setAttribute('aria-label',t('颜色 {0}',color));b.onclick=()=>setColor(color);$('swatches').append(b)}
function setColor(color){$('paint-color').value=color;$('color-hex').value=color.toUpperCase();$('color-hex').removeAttribute('aria-invalid');eraser=false;$('eraser').setAttribute('aria-pressed','false');}
$('paint-color').oninput=()=>setColor($('paint-color').value);
$('color-hex').oninput=()=>{const color=$('color-hex').value.trim();if(/^#?[0-9a-f]{6}$/i.test(color))setColor(color.startsWith('#')?color:'#'+color);};
$('color-hex').onchange=()=>{const color=$('color-hex').value.trim();if(!/^#?[0-9a-f]{6}$/i.test(color)){$('color-hex').setAttribute('aria-invalid','true');message(t('请输入六位颜色值，例如 #005A78'),true);return;}setColor(color.startsWith('#')?color:'#'+color);message('');};$('eraser').onclick=()=>{eraser=!eraser;$('eraser').setAttribute('aria-pressed',String(eraser))};
function edit(fn){stop();checkpoint();fn();render();}
$('clear-frame').onclick=()=>edit(()=>current().pixels=blank());
$('add-frame').onclick=()=>edit(()=>{frames().splice(++index,0,{durationMs:100,pixels:blank()})});
$('copy-frame').onclick=()=>edit(()=>{const f=clone(current());frames().splice(++index,0,f)});
$('delete-frame').onclick=()=>{if(frames().length>1)edit(()=>frames().splice(index,1))};
for(const [id,delta] of [['move-left',-1],['move-right',1]])$(id).onclick=()=>edit(()=>{const f=frames().splice(index,1)[0];index+=delta;frames().splice(index,0,f)});
$('frame-duration').onchange=()=>{if(frames().length===1)return;const n=Number($('frame-duration').value);if(!Number.isInteger(n)||n<100||n>60000){message(t('每帧时长应为 100–60000 毫秒'),true);$('frame-duration').value=current().durationMs;return}edit(()=>current().durationMs=n)};
$('theme-name').oninput=()=>{const name=$('theme-name').value;checkpoint();draft.name=name;mark()};
for(const [id,source,destination] of [['undo',()=>undo,()=>redo],['redo',()=>redo,()=>undo]])$(id).onclick=()=>{if(!source().length)return;stop();destination().push(JSON.stringify(draft));draft=JSON.parse(source().pop());render()};
$('play-preview').onclick=()=>{playing=!playing;started=performance.now();$('play-preview').textContent=playing?t('暂停预览'):t('播放循环');if(!playing)render()};
function animate(now){if(playing&&draft){let elapsed=(now-started)%frames().reduce((s,f)=>s+f.durationMs,0);for(let i=0;i<frames().length;i++){const f=frames()[i];if(elapsed<f.durationMs){preview(f.pixels);$('preview-time').textContent=t('第 {0} 帧 · {1} ms',i+1,f.durationMs);break}elapsed-=f.durationMs}}requestAnimationFrame(animate)}requestAnimationFrame(animate);
$('theme-list').onchange=()=>task(async()=>{const id=$('theme-list').value;if(await discard()){choose(library.themes.find(t=>t.id===id));message('')}else $('theme-list').value=draft.id});
$('new-theme').onclick=()=>task(async()=>{if(!await discard())return;const theme={version:1,id:crypto.randomUUID(),name:t('新主题'),states:Object.fromEntries(Object.keys(names).map(s=>[s,[{durationMs:1000,pixels:blank()}]]))};choose(theme);baseline='';message(t('绘制各状态的图案，然后保存。'))});
$('copy-theme').onclick=()=>task(async()=>{const theme=clone(draft);theme.id=crypto.randomUUID();theme.name=(themeName(draft.id,theme.name)+t(' 副本')).slice(0,40);choose(theme);baseline='';message(t('已复制，保存后可应用。'))});
$('save-theme').onclick=()=>task(async()=>{const theme=clone(draft);if(theme.id==='builtin'){theme.id=crypto.randomUUID();theme.name=theme.name===library.themes.find(x=>x.id==='builtin').name?(themeName(draft.id,theme.name)+t(' 副本')).slice(0,40):theme.name}await invoke('save_theme',{theme});await reload();choose(library.themes.find(x=>x.id===theme.id));message(t('主题已保存'))});
$('apply-theme').onclick=()=>task(async()=>{await invoke('activate_theme',{id:draft.id});await reload();$('theme-list').value=draft.id;message(t('已应用，键盘将跟随任务播放此主题'))});
$('import-theme').onclick=()=>task(async()=>{if(!await discard())return;const theme=await invoke('import_theme');if(theme){choose(theme);baseline='';message(t('已导入，保存后可应用。'))}});
$('export-theme').onclick=()=>task(async()=>{if(await invoke('export_theme',{theme:draft}))message(t('主题已导出'))});
(async()=>{try{await reload();choose(library.themes.find(t=>t.id===library.activeId));await window.__TAURI__.event.listen('snapshot-changed',({payload:s})=>{library.activeId=s.activeThemeId;$('active-theme').textContent=t('当前应用：{0}',themeName(s.activeThemeId,s.themeName));});const win=window.__TAURI__.window.getCurrentWindow();await win.onCloseRequested(async e=>{if(dirty()){e.preventDefault();if(await discard())await win.destroy()}})}catch(e){message(messageText(e),true)}})();

window.addEventListener('yogo-language-changed',()=>{
 for(const button of $('state-list').children)button.firstChild.nodeValue=t(names[button.dataset.state]);
 if(!library||!draft)return;
 for(const option of $('theme-list').options){
  const theme=library.themes.find(item=>item.id===option.value);
  option.textContent=theme?themeName(theme.id,theme.name)+(theme.id==='builtin'?t(' · 内置'):''):draft.name+t(' · 未保存');
 }
 $('active-theme').textContent=t('当前应用：{0}',themeName(library.activeId,library.themes.find(item=>item.id===library.activeId)?.name||'—'));
 for(const [i,button] of [...$('pixel-editor').children].entries())button.setAttribute('aria-label',t('第 {0} 行，第 {1} 列',Math.floor(i/6)+1,i%6+1));
 for(const button of $('swatches').children)button.setAttribute('aria-label',t('颜色 {0}',button.dataset.color));
 $('play-preview').textContent=playing?t('暂停预览'):t('播放循环');
 message('');render();
});
})();
