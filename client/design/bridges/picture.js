((viewer) => {
  if (window.parent === window) return;

  const address = new RegExp(viewer.address, viewer.address_flags);
  const most = viewer.most_pictures;
  const pointer = document.createElement("style");

  pointer.textContent = `${viewer.pictures}{cursor:zoom-in}`;
  document.head.append(pointer);

  const picture = (image) => {
    const src = image.currentSrc || image.src;

    if (!address.test(src) || image.getClientRects().length === 0) return null;

    const caption = image.closest("figure")?.querySelector("figcaption");

    return { src, caption: image.alt.trim() || (caption?.textContent ?? "").trim() };
  };

  addEventListener("click", (event) => {
    const target = event.target;

    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    if (!(target instanceof HTMLImageElement) || !target.matches(viewer.pictures)) return;

    const all = [];
    let at = -1;

    for (const image of document.querySelectorAll(viewer.pictures)) {
      const read = picture(image);

      if (read === null) continue;
      if (image === target) at = all.length;

      all.push(read);
    }

    if (at < 0) return;

    const from = Math.max(0, Math.min(at - most / 2, all.length - most));

    setTimeout(() => {
      if (!event.defaultPrevented) parent.postMessage({ type: viewer.message, at: at - from, pictures: all.slice(from, from + most) }, "*");
    });
  });
})
