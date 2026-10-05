(() => {
const {t, messageText, themeName} = window.YogoI18n;
// Main-window connection status. Authorization stays in Codex's own UI.
(() => {
  const $ = id => document.getElementById(id);
  const {invoke} = window.__TAURI__.core;
  let status = {state:'checking', detail:t('正在检查 Hooks 授权…')}, lastEventAt = null, checking = false, acting = false;
  const labels = {checking:'检测中…',unauthorized:'未授权',modified:'需要重新授权',legacy_plugin:'需要迁移',incomplete:'配置不完整',disabled:'已停用',missing:'未配置 Hooks',unavailable:'未找到 Codex',error:'检测失败'};
  function render() {
    const ready = status.state === 'authorized';
    $('codex-status').textContent = ready ? (lastEventAt ? t('已连接') : t('已授权 · 等待事件')) : t(labels[status.state] || '检测失败');
    $('codex-status').className = `badge ${ready ? 'green' : ['unauthorized','modified','disabled','legacy_plugin'].includes(status.state) ? 'amber' : ''}`;
    $('codex-detail').hidden = ready || ['checking','unauthorized'].includes(status.state);
    $('codex-detail').textContent = ready ? '' : messageText(status.detail);
    if(ready) $('codex-help').hidden = true;
    const action = $('codex-authorize');
    action.hidden = !['unauthorized','modified','disabled','missing','incomplete'].includes(status.state);
    action.textContent = acting ? t('正在配置…') : ({missing:t('配置 Hooks'),incomplete:t('重新配置'),disabled:t('前往启用')})[status.state] || t('前往授权');
    action.disabled = acting;
  }
  async function refresh() {
    if(checking) return;
    checking = true; render();
    try {status = await invoke('get_codex_connection');}
    catch(e) {status = {state:'error',detail:t('无法检测 Codex：{0}',messageText(e))};}
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
    $('codex-help').textContent = t('正在配置任务事件…');
    try {
      await invoke('configure_codex_hooks');
      $('codex-help').hidden = true;
      selectGuideTab('client');
      $('authorization-guide').showModal();
      await refresh();
    }
    catch(e) {$('codex-help').textContent = messageText(e);}
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
  window.addEventListener('yogo-language-changed', render);
  window.addEventListener('focus', refresh);
  document.addEventListener('visibilitychange', () => {if(!document.hidden) refresh();});
  setInterval(() => {if(!document.hidden) refresh();}, 15000);
  window.__TAURI__.event.listen('snapshot-changed', e => {lastEventAt = e.payload.tasks.lastEventAt;render();});
  invoke('get_snapshot').then(s => {lastEventAt = s.tasks.lastEventAt;render();}).catch(() => {});
  refresh();
})();

})();
