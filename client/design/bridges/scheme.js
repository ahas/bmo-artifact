((chosen) => {
  const asked = /\(\s*prefers-color-scheme\s*:\s*(dark|light)\s*\)/gi;
  const answered = (query) => String(query).replace(asked, (_, scheme) => (scheme.toLowerCase() === chosen ? "(min-width: 0px)" : "(max-width: -1px)"));

  const rules = (list) => {
    for (const rule of list) {
      if (rule.media && /prefers-color-scheme/i.test(rule.media.mediaText)) rule.media.mediaText = answered(rule.media.mediaText);
      if (rule.cssRules) rules(rule.cssRules);
    }
  };

  const sheets = () => {
    for (const sheet of document.styleSheets) {
      if (sheet.ownerNode?.hasAttribute?.("data-archive-sheet")) continue;

      try {
        rules(sheet.cssRules);
      } catch {}
    }
  };

  new MutationObserver(sheets).observe(document.documentElement, { childList: true, subtree: true });
  document.addEventListener("DOMContentLoaded", sheets);
  addEventListener("load", sheets);

  const ask = matchMedia.bind(window);

  window.matchMedia = (query) => ask(answered(query));
})
