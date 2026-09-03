mergeInto(LibraryManager.library, {
  BattlementWebLogicalPixelScale: function () {
    var canvas = Module.canvas;
    if (!canvas || canvas.clientWidth <= 0) {
      return 1;
    }
    return canvas.width / canvas.clientWidth;
  },
});
