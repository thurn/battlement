mergeInto(LibraryManager.library, {
  masonry_web_thread_count: function () {
    // init.js owns browser policy so the Rust engine receives a number rather
    // than duplicating fragile mobile user-agent checks inside WebAssembly.
    return globalThis.masonryWebThreads.threadCount;
  },
});
