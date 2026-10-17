(() => {
  const clipboard = navigator.clipboard;

  if (!clipboard || !clipboard.writeText) return;

  const write = clipboard.writeText.bind(clipboard);

  clipboard.writeText = (text) => {
    const area = document.createElement("textarea");

    area.value = String(text);
    area.setAttribute("readonly", "");
    area.style.cssText = "position:fixed;top:0;left:0;opacity:0";
    document.body.append(area);
    area.select();

    let copied = false;

    try {
      copied = document.execCommand("copy");
    } catch {}

    area.remove();

    return copied ? Promise.resolve() : write(text);
  };
})()
