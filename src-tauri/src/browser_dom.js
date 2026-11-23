(command) => {
  // This fixed function runs in the remote page. No script supplied by the model is executed.
  const clean = (text, limit = 200) => String(text || '').replace(/\s+/g, ' ').trim().slice(0, limit);
  const visible = element => !element.closest('[hidden], [aria-hidden="true"]') && getComputedStyle(element).display !== 'none' && getComputedStyle(element).visibility !== 'hidden';
  const fields = () => Array.from(document.querySelectorAll('input,textarea')).filter(input => {
    if (input.disabled || input.readOnly || !visible(input) || (!['search', 'text', ''].includes(input.type) && input.tagName !== 'TEXTAREA')) return false;
    const form = input.form;
    if (form) {
      const method = (form.method || 'get').toLowerCase();
      const action = new URL(form.getAttribute('action') || location.href, location.href);
      const postSearch = method === 'post' && (/^\/search(?:\/|$)|^\/arama(?:\/|$)/i.test(action.pathname) || /(^|\.)duckduckgo\.com$/.test(action.hostname));
      // Google's text search form also contains an empty, hidden Lens file picker.
      // Only its same-origin GET /search q field may coexist with an EMPTY picker.
      const googleTextSearch = method === 'get' && input.name === 'q' && action.pathname === '/search' && action.origin === location.origin && /^(www\.)?google\.[a-z.]+$/.test(location.hostname);
      const unsafeFile = Array.from(form.querySelectorAll('input[type=file]')).some(file => !googleTextSearch || file.files?.length > 0);
      if ((method !== 'get' && !postSearch) || unsafeFile || form.querySelector('input[type=password],input[type=email]')) return false;
    }
    const label = [input.type, input.name, input.id, input.placeholder, input.getAttribute('aria-label')].join(' ');
    return input.type === 'search' || /(^|\s)(q|s|query|search|searchbox|arama|ara)(\s|$|[-_])/i.test(label) || Boolean(input.closest('[role=search]'));
  }).slice(0, 8);
  try {
    if (command.action === 'search') {
      if (location.href !== command.expectedUrl) return { error: 'Sayfa değişti. Arama yeniden planlanmalı.' };
      const input = fields()[command.index];
      if (!input) return { error: 'Site arama alanı artık bulunamıyor.' };
      const form = input.form;
      if (form) {
        const action = new URL(form.getAttribute('action') || location.href, location.href);
        if (action.protocol !== 'https:' || action.origin !== location.origin) return { error: 'Arama başka bir siteye gönderilemez.' };
      }
      input.scrollIntoView({ block: 'center' });
      input.focus();
      const prototype = input.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(prototype, 'value').set.call(input, command.query);
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
      if (form) {
        // Only recognized search forms reach here; credentials and upload forms are excluded.
        const button = Array.from(form.querySelectorAll('button,input[type=submit]')).find(button => !button.disabled && /search|ara|submit|go/i.test([button.type, button.innerText, button.value, button.getAttribute('aria-label')].join(' ')));
        form.requestSubmit(button?.type === 'submit' ? button : undefined);
      } else {
        input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true }));
        input.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true }));
      }
      return { ok: true };
    }
    const googleSearch = /^(www\.)?google\.[a-z.]+$/.test(location.hostname);
    const engine = /(^|\.)(duckduckgo\.com|bing\.com)$/.test(location.hostname) || googleSearch;
    const results = engine ? Array.from(document.querySelectorAll('a.result__a, a[data-testid="result-title-a"], article h2 a, #search a:has(h3), .b_algo h2 a')) : [];
    const anchors = results.length ? results : Array.from(document.querySelectorAll('main a[href], article a[href], [role=main] a[href], body a[href]'));
    const seen = new Set();
    const links = anchors.filter(anchor => {
      if (!visible(anchor) || !clean(anchor.innerText || anchor.getAttribute('aria-label') || anchor.textContent) || !anchor.href.startsWith('https:') || seen.has(anchor.href)) return false;
      seen.add(anchor.href); return true;
    }).slice(0, 40).map((anchor, index) => ({ index, title: clean(anchor.innerText || anchor.getAttribute('aria-label') || anchor.textContent, 180), url: anchor.href }));
    const root = document.querySelector('article, main, [role=main]') || document.body;
    const copy = root?.cloneNode(true);
    copy?.querySelectorAll('script,style,noscript,nav,header,footer,form,input,textarea,button,svg,[hidden],[aria-hidden=true]').forEach(element => element.remove());
    const text = clean(copy?.textContent, 12000);
    const heading = clean(document.title).toLowerCase();
    const google = /(^|\.)google\.[a-z.]+$/.test(location.hostname);
    const blocked = /just a moment|attention required|verify you are human|access denied|robot check|captcha/.test(heading) || (google && (location.pathname.startsWith('/sorry') || location.hostname.startsWith('consent.') || (googleSearch && results.length === 0 && /unusual traffic from your computer network|bilgisayar ağınızdan gelen olağan dışı trafik/i.test(text)))) || (text.length < 1500 && /complete the security check|confirm you are human|unfortunately, bots use duckduckgo|select all squares containing|insan olduğunuzu doğrula/i.test(text));
    return { url: location.href, title: clean(document.title), text, links, searchFields: fields().map((input, index) => ({ index, label: clean(input.getAttribute('aria-label') || input.placeholder || input.name || 'Ara') })), searchResults: engine, blocked, ready: document.readyState };
  } catch (_) { return { error: 'Sayfa okunamadı.' }; }
}
