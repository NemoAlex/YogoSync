(() => {
const {t, messageText, themeName} = window.YogoI18n;
// Shared optical approximation for the home screen and theme editor.
window.YogoGlow = {
  render(element, pixels) {
    const colors = pixels.flat();
    [...element.children].forEach((dot, i) => {
      const color = colors[i];
      dot.classList.toggle('is-lit', color.some(Boolean));
      dot.style.setProperty('--light', color.join(','));
      const peak = Math.max(...color);
      // Tint the translucent white diffuser with normalized emission; near-black
      // LEDs contribute near-zero opacity instead of dark paint.
      dot.style.setProperty('--emission', color.map(c => peak ? Math.round(c * 255 / peak) : 0).join(','));
      dot.style.setProperty('--emission-alpha', String(Math.min(.78, Math.pow(peak / 180, 1.5))));
    });
  }
};

// This is a local hardware appearance preference, separate from theme pixels.
(() => {
  const key = 'yogo-preview-cover';
  const choices = ['black', 'white', 'yellow'];
  function apply(value) {
    const cover = choices.includes(value) ? value : 'black';
    document.documentElement.dataset.cover = cover;
    document.querySelectorAll('[data-cover]').forEach(button => {
      if (button.tagName === 'BUTTON') button.setAttribute('aria-pressed', String(button.dataset.cover === cover));
    });
  }
  function restore() {
    try { apply(localStorage.getItem(key)); } catch { apply('black'); }
  }
  const buttons = [...document.querySelectorAll('button[data-cover]')];
  const native = window.__TAURI__;
  const busy = value => buttons.forEach(button => { button.disabled = value; });
  let saving = false;
  async function refresh() {
    if (saving) return;
    if (native) apply((await native.core.invoke('get_snapshot')).preferences.previewCover);
    else restore();
  }
  const error = document.createElement('p');
  error.setAttribute('role', 'status');
  error.style.cssText = 'color:var(--red);font-size:11px;margin:4px 0';
  error.hidden = true;
  document.querySelector('.cover-picker')?.after(error);
  buttons.forEach(button => {
    button.addEventListener('click', async () => {
      saving = true; busy(true); error.hidden = true;
      try {
        if (native) await native.core.invoke('set_preview_cover', {color:button.dataset.cover});
        else localStorage.setItem(key, button.dataset.cover);
        apply(button.dataset.cover);
      } catch (e) {
        error.textContent = t('屏罩颜色保存失败：') + messageText(e); error.hidden = false;
      } finally { saving = false; busy(false); }
    });
  });
  if (native) {
    busy(true);
    (async () => {
      await native.event.listen('snapshot-changed', event => {
        if (!saving) apply(event.payload.preferences.previewCover);
      });
      await refresh();
    })().catch(e => {
      error.textContent = t('屏罩颜色读取失败：') + messageText(e); error.hidden = false;
    }).finally(() => busy(false));
  } else restore();
  window.addEventListener('storage', event => { if (!native && (event.key === key || event.key === null)) restore(); });
  window.addEventListener('focus', () => { refresh().catch(() => {}); });
})();

})();
