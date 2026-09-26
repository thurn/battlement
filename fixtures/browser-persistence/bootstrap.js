Module['preRun'] = [function () {
  FS.mkdir('/idbfs');
  FS.mount(IDBFS, { autoPersist: true }, '/idbfs');
  addRunDependency('persistent-files');
  FS.syncfs(true, function (error) {
    if (error) throw error;
    removeRunDependency('persistent-files');
  });
}];

Module['onRuntimeInitialized'] = function () {
  window.fixture = {
    reset: function () { return Module['_fixture_reset'](); },
    update: function (index, value) { Module['_fixture_update'](index, value); },
    clear: function (index) { Module['_fixture_clear'](index); },
    retry: function (index) { Module['_fixture_retry'](index); },
    snapshot: function (index) {
      return JSON.parse(UTF8ToString(Module['_fixture_snapshot'](index)));
    },
    bridge: BattlementPersistence,
    fs: FS,
    idbfs: IDBFS,
  };
};
