mergeInto(LibraryManager.library, {
  BattlementMotionProfilePublish: function (jsonPointer) {
    document.documentElement.setAttribute(
      "data-battlement-motion-profile",
      UTF8ToString(jsonPointer)
    );
  },
});
