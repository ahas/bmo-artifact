(() => {
  if (window.parent === window) return;

  addEventListener("click", (event) => {
    const link = event.target instanceof Element ? event.target.closest("a[href]") : null;

    if (event.defaultPrevented || !link || (link.target !== "" && link.target !== "_self")) return;
    if ((link.getAttribute("href") ?? "").startsWith("#") || link.protocol === "javascript:") return;

    link.target = "_blank";
  });
})()
