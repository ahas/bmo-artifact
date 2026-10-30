((sounds) => {
  const kept = "attachment:";
  const spent = new Set(["src", "controls", "preload", "autoplay", "crossorigin"]);
  const written = new WeakMap();

  const found = (address) => {
    if (address === null || !address.startsWith(kept)) return address === null ? null : { play: address, download: address, name: null };

    let name = address.slice(kept.length);

    try {
      name = decodeURIComponent(name);
    } catch {}

    const file = sounds.files[name];

    return file === undefined ? { play: address, download: address, name } : { ...file, name };
  };

  const audioOf = (element) => (element.localName === "audio" ? element : element.localName === "source" ? element.closest("audio") : null);
  const addressOf = (audio) => audio.getAttribute("src") ?? audio.querySelector("source[src]")?.getAttribute("src") ?? null;

  const point = (audio) => {
    if (written.has(audio)) return;

    const sound = found(addressOf(audio));

    if (sound === null) return;

    written.set(audio, sound);
    if (sound.play !== addressOf(audio)) audio.src = sound.play;
  };

  const pointing = new MutationObserver((changes) => {
    for (const change of changes) {
      for (const node of change.addedNodes) {
        const audio = node instanceof Element ? audioOf(node) : null;

        if (audio !== null) point(audio);
      }
    }
  });

  pointing.observe(document.documentElement, { childList: true, subtree: true });

  document.addEventListener("DOMContentLoaded", () => {
    pointing.disconnect();

    const player = window.BmoAudioPlayer;

    player?.define({ words: sounds.words, locale: sounds.locale });

    for (const audio of [...document.querySelectorAll("audio")]) {
      point(audio);

      const sound = written.get(audio) ?? null;

      if (player === undefined) continue;

      const element = document.createElement(player.AUDIO_PLAYER_TAG);

      for (const { name, value } of [...audio.attributes]) {
        if (!spent.has(name)) element.setAttribute(name, value);
      }

      if (sound !== null) {
        element.setAttribute("src", sound.play);
        element.setAttribute("download", sound.download);
      }

      const named = audio.getAttribute("title") ?? sound?.name ?? null;

      if (named !== null) element.setAttribute("name", named);
      if (sounds.scheme !== null) element.setAttribute("scheme", sounds.scheme);

      audio.pause();
      audio.replaceWith(element);
    }
  });
})
