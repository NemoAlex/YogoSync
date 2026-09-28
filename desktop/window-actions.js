(() => {
  const {invoke} = window.__TAURI__.core;
  let lastDirty, pending = Promise.resolve();
  window.YogoWindow = {
    setDirty(value) {
      if (value === lastDirty) return pending;
      lastDirty = value;
      pending = pending.then(() => invoke('set_unsaved_changes', {dirty:value})).catch(() => {lastDirty = undefined;});
      return pending;
    }
  };
  // Native menu accelerators are primary; handle WebView-focused shortcuts too.
  document.addEventListener('keydown', async event => {
    const modifier = /Mac/i.test(navigator.platform) ? event.metaKey : event.ctrlKey;
    const key = event.key.toLowerCase();
    if (!modifier || event.altKey || event.shiftKey || !['w','q'].includes(key)) return;
    event.preventDefault();
    if (event.repeat) return;
    document.activeElement?.blur();
    await pending;
    await invoke(key === 'q' ? 'quit_app' : 'close_current_window');
  });
})();
