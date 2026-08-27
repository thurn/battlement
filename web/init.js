(() => {
  const resetParameter = "battlement-clear-storage";
  const url = new URL(window.location.href);
  let compatibilityErrorShown = false;

  const clientHintsMobile =
    navigator.userAgentData && navigator.userAgentData.mobile;
  const mobileUserAgent =
    /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini|Mobile|Silk/i.test(
      navigator.userAgent,
    );
  // Modern iPadOS can request a desktop user agent, but it still uses mobile
  // WebKit and has the same synchronous pthread-startup failure as iOS Safari.
  const iPadDesktopMode =
    navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1;
  const isMobile = Boolean(
    clientHintsMobile || mobileUserAgent || iPadDesktopMode,
  );
  const hardwareConcurrency = Math.max(
    1,
    navigator.hardwareConcurrency || 1,
  );
  // Leave one logical processor for the browser and cap this sample's search
  // pool. Mobile deliberately selects one so Rayon never creates a Web Worker.
  const desktopThreadCount = Math.max(
    1,
    Math.min(4, hardwareConcurrency - 1),
  );
  const threadCount = isMobile ? 1 : desktopThreadCount;

  function showWebThreadsError() {
    compatibilityErrorShown = true;

    // A threaded Unity build cannot fall back after its Wasm module starts.
    // Keep the capability test and user-facing failure state here, while the
    // generated Unity entry point only decides whether it is safe to bootstrap.
    if (!document.querySelector('meta[name="viewport"]')) {
      const viewport = document.createElement("meta");
      viewport.name = "viewport";
      viewport.content = "width=device-width, initial-scale=1.0";
      document.head.appendChild(viewport);
    }

    const warningBanner = document.querySelector("#unity-warning");
    const error = document.createElement("div");
    error.textContent =
      "This game requires SharedArrayBuffer and cross-origin isolation.";
    error.style = "max-width: 32rem; text-align: center;";
    warningBanner.appendChild(error);
    document.body.replaceChildren(warningBanner);
    warningBanner.style =
      "position: fixed; inset: 0; display: flex; align-items: center; " +
      "justify-content: center; transform: none; padding: 24px; " +
      "box-sizing: border-box; z-index: 2147483647; background: #1f1f20; " +
      "color: white; font: 18px/1.5 Arial, sans-serif;";
  }

  // init.js runs from the document head before Unity's generated startup script.
  // Threaded builds use this result to avoid requesting the loader at all when
  // SharedArrayBuffer is unavailable; non-threaded builds simply ignore it.
  window.battlementWebThreads = Object.freeze({
    isSupported:
      self.crossOriginIsolated && typeof SharedArrayBuffer !== "undefined",
    isMobile,
    // Unity creates its own persistent workers from hardwareConcurrency. Desktop
    // needs additional prestarted workers for Rayon's dedicated pool; mobile's
    // current-thread pool needs no surplus worker and must not attempt to make one.
    pthreadPoolSize: hardwareConcurrency + (isMobile ? 0 : threadCount),
    showUnsupportedError: showWebThreadsError,
    threadCount,
  });

  function addShellStyle() {
    const style = document.createElement("style");
    style.textContent = `
      html, body {
        background: #03070c;
        height: 100%;
        margin: 0;
        overflow: hidden;
        width: 100%;
      }
      #unity-container,
      #unity-container.unity-desktop,
      #unity-container.unity-mobile {
        align-items: center;
        display: flex;
        height: 100vh !important;
        inset: 0 !important;
        justify-content: center;
        position: fixed !important;
        transform: none !important;
        width: 100vw !important;
      }
      #unity-canvas {
        display: block;
        height: var(--battlement-canvas-height, 720px) !important;
        max-height: 100vh;
        max-width: 100vw;
        width: var(--battlement-canvas-width, 1280px) !important;
      }
      #unity-canvas:focus-visible {
        outline: none;
      }
      #unity-footer { display: none; }
    `;
    document.head.appendChild(style);
  }

  function fitCanvas() {
    const scale = Math.min(window.innerWidth / 1280, window.innerHeight / 720);
    document.documentElement.style.setProperty(
      "--battlement-canvas-width",
      `${1280 * scale}px`,
    );
    document.documentElement.style.setProperty(
      "--battlement-canvas-height",
      `${720 * scale}px`,
    );
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
      "<!doctype html><title>Resetting Battlement</title>" +
        "<style>body{background:#121216;color:#fff;font:16px system-ui;display:grid;place-items:center;height:100vh;margin:0}</style>" +
        "<p id='battlement-reset-status'>Clearing browser storage…</p>",
    );
    document.close();
    url.searchParams.delete(resetParameter);
    clearStorage()
      .then(() => window.location.replace(url.toString()))
      .catch((error) => {
        document.querySelector("#battlement-reset-status").textContent =
          `Could not clear storage: ${error.message} ` +
          "Close other Battlement tabs and try again.";
      });
    return;
  }

  addShellStyle();
  fitCanvas();
  window.addEventListener("resize", fitCanvas);
  window.addEventListener("DOMContentLoaded", () => {
    if (compatibilityErrorShown) {
      return;
    }
    const canvas = document.querySelector("#unity-canvas");
    if (canvas) {
      canvas.tabIndex = 0;
      canvas.setAttribute("aria-label", "Battlement game");
      canvas.addEventListener(
        "keydown",
        (event) => {
          if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", " "].includes(event.key)) {
            event.preventDefault();
          }
        },
        true,
      );
      canvas.focus({ preventScroll: true });
    }
  });
})();
