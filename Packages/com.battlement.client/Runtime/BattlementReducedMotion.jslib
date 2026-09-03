mergeInto(LibraryManager.library, {
  BattlementPrefersReducedMotion: function () {
    if (typeof matchMedia !== "function") {
      return -1;
    }
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return 1;
    }
    return matchMedia("(prefers-reduced-motion: no-preference)").matches ? 0 : -1;
  },
});
