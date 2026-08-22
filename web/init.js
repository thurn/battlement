(() => {
  const resetParameter = "masonry-clear-storage";
  const url = new URL(window.location.href);

  function addResetStyle() {
    const style = document.createElement("style");
    style.textContent = `
      #masonry-storage-reset {
        align-items: center;
        background: rgba(18, 18, 22, 0.82);
        border: 1px solid rgba(255, 255, 255, 0.28);
        border-radius: 10px;
        color: white;
        cursor: pointer;
        display: flex;
        height: 42px;
        justify-content: center;
        left: max(12px, env(safe-area-inset-left));
        padding: 0;
        position: fixed;
        top: max(12px, env(safe-area-inset-top));
        transition: background 120ms ease, transform 120ms ease;
        width: 42px;
        z-index: 2147483647;
      }
      #masonry-storage-reset:hover { background: rgba(38, 38, 44, 0.94); }
      #masonry-storage-reset:active { transform: scale(0.94); }
      #masonry-storage-reset:focus-visible {
        outline: 3px solid #70b7ff;
        outline-offset: 2px;
      }
    `;
    document.head.appendChild(style);
  }

  function deleteDatabase(name) {
    return new Promise((resolve, reject) => {
      const request = indexedDB.deleteDatabase(name);
      request.onsuccess = resolve;
      request.onerror = () => reject(request.error);
      request.onblocked = () =>
        reject(new Error(`Database ${name} is open in another tab.`));
    });
  }

  async function clearStorage() {
    localStorage.clear();
    sessionStorage.clear();
    const operations = [];
    const databaseNames = ["/idbfs", "UnityCache"];
    if (window.caches) {
      operations.push(
        caches
          .keys()
          .then((keys) => Promise.all(keys.map((key) => caches.delete(key)))),
      );
    }
    if (indexedDB.databases) {
      const databases = await indexedDB.databases();
      databaseNames.push(
        ...databases
          .filter((database) => database.name)
          .map((database) => database.name),
      );
    }
    operations.push(
      Promise.all([...new Set(databaseNames)].map(deleteDatabase)),
    );
    await Promise.all(operations);
  }

  if (url.searchParams.has(resetParameter)) {
    document.open();
    document.write(
      "<!doctype html><title>Resetting Masonry</title>" +
        "<style>body{background:#121216;color:#fff;font:16px system-ui;display:grid;place-items:center;height:100vh;margin:0}</style>" +
        "<p id='masonry-reset-status'>Clearing browser storage…</p>",
    );
    document.close();
    url.searchParams.delete(resetParameter);
    clearStorage()
      .then(() => window.location.replace(url.toString()))
      .catch((error) => {
        document.querySelector("#masonry-reset-status").textContent =
          `Could not clear storage: ${error.message} ` +
          "Close other Masonry tabs and try again.";
      });
    return;
  }

  addResetStyle();
  window.addEventListener("DOMContentLoaded", () => {
    const button = document.createElement("button");
    button.id = "masonry-storage-reset";
    button.type = "button";
    button.title = "Clear browser storage and reload";
    button.setAttribute("aria-label", button.title);
    button.innerHTML =
      '<svg aria-hidden="true" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">' +
      '<path d="M3 6h18"/><path d="M8 6V4h8v2"/>' +
      '<path d="M19 6l-1 14H6L5 6"/><path d="M10 11v5"/>' +
      '<path d="M14 11v5"/></svg>';
    button.addEventListener("click", () => {
      url.searchParams.set(resetParameter, "1");
      window.location.assign(url.toString());
    });
    document.body.appendChild(button);
  });
})();
