// Main-window connection status. Authorization stays in Codex's own UI.
(() => {
  const $ = id => document.getElementById(id);
  const {invoke} = window.__TAURI__.core;
  let status = {state:'checking', detail:'正在检查 Codex 插件授权…'}, lastEventAt = null, checking = false, acting = false;
  const labels = {checking:'检测中…',unauthorized:'未授权',modified:'需要重新授权',plugin_disabled:'插件已停用',disabled:'已停用',missing:'未安装插件',unavailable:'未找到 Codex',error:'检测失败'};
  function render() {
    const ready = status.state === 'authorized';
    $('codex-status').textContent = ready ? (lastEventAt ? '已连接' : '已授权 · 等待事件') : labels[status.state] || '检测失败';
    $('codex-status').className = `badge ${ready ? 'green' : ['unauthorized','modified','disabled','plugin_disabled'].includes(status.state) ? 'amber' : ''}`;
    $('codex-detail').hidden = ready || ['checking','unauthorized'].includes(status.state);
    $('codex-detail').textContent = ready ? '' : status.detail;
    if(ready) $('codex-help').hidden = true;
    const action = $('codex-authorize');
    action.hidden = !['unauthorized','modified','disabled','plugin_disabled','missing'].includes(status.state);
    action.textContent = acting ? '正在打开…' : ({missing:'安装插件',plugin_disabled:'前往启用',disabled:'前往启用'})[status.state] || '前往授权';
    action.disabled = acting;
  }
  async function refresh() {
    if(checking) return;
    checking = true; render();
    try {status = await invoke('get_codex_connection');}
    catch(e) {status = {state:'error',detail:`无法检测 Codex：${String(e)}`};}
    finally {checking = false;render();}
  }
  $('codex-authorize').onclick = async () => {
    if(acting) return;
    const state = status.state;
    if(['unauthorized','modified','disabled'].includes(state)) {
      $('codex-help').hidden = true;
      selectGuideTab('client');
      $('authorization-guide').showModal();
      return;
    }
    acting = true; render();
    $('codex-help').hidden = false;
    const installing = state === 'missing';
    $('codex-help').textContent = installing
      ? '正在准备插件文件。请在打开的 Codex 插件页完成安装，随后返回这里进行授权。'
      : '请在 Codex 插件页启用 YOGO Pet，完成后返回这里。';
    try {await invoke(installing ? 'install_codex_plugin' : 'open_codex_plugin');}
    catch(e) {$('codex-help').textContent = String(e);}
    finally {acting = false;render();}
  };
  function selectGuideTab(selected, focus = false) {
    for(const name of ['client','cli']) {
      const active = name === selected, tab = $(`guide-tab-${name}`);
      tab.setAttribute('aria-selected', String(active));
      tab.tabIndex = active ? 0 : -1;
      $(`guide-panel-${name}`).hidden = !active;
      if(active && focus) tab.focus();
    }
    $('guide-panel-client').parentElement.scrollTop = 0;
  }
  for(const name of ['client','cli']) {
    $(`guide-tab-${name}`).onclick = () => selectGuideTab(name);
    $(`guide-tab-${name}`).onkeydown = event => {
      if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key)) return;
      event.preventDefault();
      selectGuideTab(event.key === 'Home' ? 'client' : event.key === 'End' ? 'cli' : name === 'client' ? 'cli' : 'client', true);
    };
  }
  $('guide-close').onclick = () => $('authorization-guide').close();
  $('guide-done').onclick = () => { $('authorization-guide').close(); refresh(); };
  window.addEventListener('focus', refresh);
  document.addEventListener('visibilitychange', () => {if(!document.hidden) refresh();});
  setInterval(() => {if(!document.hidden) refresh();}, 15000);
  window.__TAURI__.event.listen('snapshot-changed', e => {lastEventAt = e.payload.tasks.lastEventAt;render();});
  invoke('get_snapshot').then(s => {lastEventAt = s.tasks.lastEventAt;render();}).catch(() => {});
  refresh();
})();
