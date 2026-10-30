((sounds) => {
  const kept = "attachment:";
  const spent = new Set(["src", "controls", "preload", "autoplay", "crossorigin"]);

  const found = (written) => {
    if (written === null || !written.startsWith(kept)) return written === null ? null : { play: written, download: written, name: null };

    let name = written.slice(kept.length);

    try {
      name = decodeURIComponent(name);
    } catch {}

    const file = sounds.files[name];

    return file === undefined ? { play: written, download: written, name } : { ...file, name };
  };

  document.addEventListener("DOMContentLoaded", () => {
    const player = window.BmoAudioPlayer;

    player?.define({ words: sounds.words, locale: sounds.locale });

    for (const audio of [...document.querySelectorAll("audio")]) {
      const sound = found(audio.getAttribute("src") ?? audio.querySelector("source[src]")?.getAttribute("src") ?? null);

      if (player === undefined) {
        if (sound !== null) audio.src = sound.play;
        continue;
      }

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

      audio.replaceWith(element);
    }
  });
})
