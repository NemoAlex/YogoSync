// Chinese source strings are stable keys; English translations live in locales/en.json.
(() => {
  const scripts = document.currentScript?.dataset.scripts?.split(',') || [];
  function resolveLocale(language) {
    return /^zh(?:[-_.]|$)/i.test(language || '') ? 'zh-CN' : 'en';
  }
  function create(locale, catalog) {
    locale = resolveLocale(locale);
    const t = (key, ...values) => (locale === 'en' ? catalog[key] ?? key : key)
      .replace(/\{(\d+)\}/g, (match, index) => values[index] === undefined ? match : String(values[index]));
    // Legacy native diagnostics arrive as text. Match complete catalog templates,
    // leaving OS errors, file paths, and user-authored text untouched.
    const patterns = Object.entries(catalog).filter(([key]) => /\{[^}]+\}/.test(key)).map(([key, value]) => {
      const names = [];
      const source = key.split(/(\{[^}]+\})/).map(part => {
        if (/^\{[^}]+\}$/.test(part)) { names.push(part); return '([\\s\\S]+?)'; }
        return part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      }).join('');
      return {expression: new RegExp('^' + source + '$'), names, value};
    });
    function messageText(value) {
      const text = String(value ?? '');
      if (locale !== 'en') return text;
      if (catalog[text] !== undefined) return catalog[text];
      for (const {expression, names, value} of patterns) {
        const match = text.match(expression);
        if (match) return value.replace(/\{[^}]+\}/g, token => {
          const index = names.indexOf(token);
          return index < 0 ? token : messageText(match[index + 1]);
        });
      }
      return text;
    }
    return {get locale() { return locale; }, setLocale(value) { locale = resolveLocale(value); }, t, messageText, themeName: (id, name) => id === 'builtin' && name === '机器人与符号' ? t('机器人与符号') : name};
  }
  window.YogoI18nFactory = {resolveLocale, create};
  if (!scripts.length) return;
  window.YogoI18nReady = (async () => {
    let language = navigator.languages?.[0] || navigator.language;
    if (window.__TAURI__) language = await window.__TAURI__.core.invoke('get_locale');
    const response = await fetch('locales/en.json');
    if (!response.ok) throw new Error('Could not load language catalog');
    const api = create(language, await response.json());
    window.YogoI18n = api;
    function applyLanguage(language) {
      api.setLocale(language);
    document.documentElement.lang = api.locale;
    document.querySelectorAll('[data-i18n]').forEach(node => { node.textContent = api.t(node.dataset.i18n); });
    for (const attribute of ['aria-label', 'placeholder', 'alt']) {
      document.querySelectorAll(`[data-i18n-${attribute}]`).forEach(node => {
        node.setAttribute(attribute, api.t(node.getAttribute(`data-i18n-${attribute}`)));
      });
    }
      window.dispatchEvent(new Event('yogo-language-changed'));
    }
    applyLanguage(language);
    if (window.__TAURI__) {
      await window.__TAURI__.event.listen('language-changed', event => applyLanguage(event.payload));
      // Re-read after subscribing so a change during startup cannot be missed.
      applyLanguage(await window.__TAURI__.core.invoke('get_locale'));
    }
    for (const src of scripts) {
      await new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = src; script.onload = resolve; script.onerror = reject;
        document.body.append(script);
      });
    }
    applyLanguage(api.locale);
    document.documentElement.classList.remove('i18n-loading');
  })().catch(error => {
    document.documentElement.classList.remove('i18n-loading');
    const notice = document.createElement('p');
    notice.setAttribute('role', 'alert');
    notice.textContent = 'Unable to load YogoSync / 无法加载 YogoSync: ' + String(error);
    document.body.prepend(notice);
    console.error(error);
  });
})();
