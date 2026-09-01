mergeInto(LibraryManager.library, {
  BattlementPrefersReducedMotion: function () {
    if (typeof matchMedia !== "function") {
      throw new Error("The browser reduced-motion bridge is unavailable.");
    }
    return matchMedia("(prefers-reduced-motion: reduce)").matches ? 1 : 0;
  },
});
