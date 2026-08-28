mergeInto(LibraryManager.library, {
  battlement_web_thread_count: function () {
    // init.js owns browser policy so the Rust engine receives a number rather
    // than duplicating fragile mobile user-agent checks inside WebAssembly.
    return globalThis.battlementWebThreads.threadCount;
  },
  BattlementWebLogSync: function () {
    FS.syncfs(false, function (error) {
      if (error) {
        console.error("Battlement log persistence failed", error);
      }
    });
  },
  BattlementConsumeRestartShortcut: function () {
    if (!globalThis.battlementWebInput) {
      return 0;
    }
    return globalThis.battlementWebInput.consumeRestartShortcut();
  },
});
