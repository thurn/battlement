mergeInto(LibraryManager.library, {
  $BattlementPersistence__deps: ['$FS', '$IDBFS', '$PATH'],
  $BattlementPersistence__postset: 'BattlementPersistence.install();',
  $BattlementPersistence: {
    queue: [],
    busy: false,
    suppressAutomatic: false,
    automaticQueue: null,
    install: function () {
      var originalSync = IDBFS.syncfs;
      BattlementPersistence.guardAutomatic();
      IDBFS.syncfs = function (mount, populate, callback) {
        BattlementPersistence.enqueue(function (done) {
          if (populate) originalSync(mount, true, done);
          else BattlementPersistence.flush(mount, done);
        }, callback);
      };
    },
    guardAutomatic: function () {
      var bridge = BattlementPersistence;
      if (IDBFS.queuePersist === bridge.automaticQueue) return;
      var originalQueue = IDBFS.queuePersist;
      bridge.automaticQueue = function (mount) {
        if (!bridge.suppressAutomatic) originalQueue(mount);
      };
      IDBFS.queuePersist = bridge.automaticQueue;
    },
    enqueue: function (operation, callback) {
      BattlementPersistence.queue.push([operation, callback]);
      BattlementPersistence.drain();
    },
    drain: function () {
      var bridge = BattlementPersistence;
      if (bridge.busy || !bridge.queue.length) return;
      bridge.busy = true;
      var item = bridge.queue.shift();
      var settled = false;
      function done(error, bytes) {
        if (settled) return;
        settled = true;
        try { item[1](error, bytes); }
        finally {
          bridge.busy = false;
          bridge.drain();
        }
      }
      try { item[0](done); }
      catch (error) { done(error); }
    },
    // IDBFS's request-error callback may precede transaction abort. Only the
    // transaction's terminal event can acknowledge an atomic durable snapshot.
    flush: function (mount, done) {
      IDBFS.getLocalSet(mount, function (error, local) {
        if (error) return done(error);
        IDBFS.getRemoteSet(mount, function (error, remote) {
          if (error) return done(error);
          var writes = [];
          var removes = [];
          try {
            Object.keys(local.entries).sort().forEach(function (path) {
              var previous = remote.entries[path];
              if (previous && previous.timestamp.getTime() === local.entries[path].timestamp.getTime()) return;
              IDBFS.loadLocalEntry(path, function (error, entry) {
                if (error) throw error;
                // MEMFS contents can change during the asynchronous transaction.
                if (entry.contents) entry.contents = entry.contents.slice();
                writes.push([path, entry]);
              });
            });
            removes = Object.keys(remote.entries).filter(function (path) {
              return !local.entries[path];
            }).sort().reverse();
            if (!writes.length && !removes.length) return done(null);
            var transaction = remote.db.transaction([IDBFS.DB_STORE_NAME], 'readwrite');
            transaction.oncomplete = function () { done(null); };
            transaction.onabort = function () {
              done(transaction.error || new Error('Persistent storage transaction aborted'));
            };
            var store = transaction.objectStore(IDBFS.DB_STORE_NAME);
            try {
              writes.forEach(function (value) { store.put(value[1], value[0]); });
              removes.forEach(function (path) { store.delete(path); });
            } catch (error) {
              transaction.onabort = function () { done(error); };
              transaction.abort();
            }
          } catch (error) { done(error); }
        });
      });
    },
    mount: function (path) {
      var node = FS.lookupPath(PATH.dirname(path)).node;
      if (node.mount.type !== IDBFS) throw new Error('Persistent path is not on IDBFS');
      return node.mount;
    },
    load: function (mount, path, done) {
      IDBFS.getDB(mount.mountpoint, function (error, db) {
        if (error) return done(error);
        try {
          var transaction = db.transaction([IDBFS.DB_STORE_NAME], 'readonly');
          var request = transaction.objectStore(IDBFS.DB_STORE_NAME).get(path);
          transaction.onabort = function () {
            done(transaction.error || new Error('Persistent storage load aborted'));
          };
          transaction.oncomplete = function () {
            var entry = request.result;
            if (entry && !FS.isFile(entry.mode)) return done(new Error('Persistent path is not a file'));
            done(null, entry ? entry.contents : null);
          };
        } catch (error) { done(error); }
      });
    },
    mutate: function (mount, path, kind, bytes, done) {
      var bridge = BattlementPersistence;
      var previous = null;
      var changed = false;
      try {
        IDBFS.loadLocalEntry(path, function (error, entry) {
          if (error && error.errno !== 44) throw error; // ENOENT
          if (entry) {
            if (!FS.isFile(entry.mode)) throw new Error('Persistent path is not a file');
            previous = { mode: entry.mode, timestamp: entry.timestamp, contents: entry.contents.slice() };
          }
        });
        bridge.suppressAutomatic = true;
        try {
          changed = true;
          if (kind === 1) {
            FS.writeFile(path, bytes);
            // IDBFS compares timestamps, including two saves in the same millisecond.
            var timestamp = new Date(Math.max(Date.now(), previous ? previous.timestamp.getTime() + 1 : 0));
            FS.utime(path, timestamp, timestamp);
          } else if (previous) FS.unlink(path);
        } finally { bridge.suppressAutomatic = false; }
      } catch (error) { return restore(error); }
      try {
        bridge.flush(mount, function (error) {
          if (error) restore(error);
          else done(null);
        });
      } catch (error) { restore(error); }
      function restore(error) {
        if (!changed) return done(error);
        bridge.suppressAutomatic = true;
        try {
          if (previous) {
            IDBFS.storeLocalEntry(path, previous, function (restoreError) {
              if (restoreError) throw restoreError;
            });
          } else if (FS.analyzePath(path).exists) FS.unlink(path);
        } catch (restoreError) {
          error = new Error(String(error) + '; restoring prior file failed: ' + String(restoreError));
        } finally { bridge.suppressAutomatic = false; }
        done(error);
      }
    },
    start: function (path, kind, bytes, done) {
      // Unity replaces queuePersist in preRun, after library postsets execute.
      BattlementPersistence.guardAutomatic();
      BattlementPersistence.enqueue(function (complete) {
        var mount = BattlementPersistence.mount(path);
        if (kind === 0) BattlementPersistence.load(mount, path, complete);
        else BattlementPersistence.mutate(mount, path, kind, bytes, complete);
      }, done);
    },
  },
  battlement_persistence_start__deps: ['$BattlementPersistence', '$UTF8ToString', '$lengthBytesUTF8', '$stringToUTF8', 'malloc', 'free'],
  battlement_persistence_start: function (path, kind, bytes, length, owner, callback) {
    // Rust's application lives on Unity's main thread; worker jobs never own stores.
    var copiedPath = UTF8ToString(path);
    var copiedBytes = HEAPU8.slice(bytes, bytes + length);
    BattlementPersistence.start(copiedPath, kind, copiedBytes, function (error, result) {
      var status = error ? -1 : result == null ? 0 : 1;
      var size = error ? lengthBytesUTF8(String(error)) : result ? result.length : 0;
      var pointer = _malloc(size + 1);
      if (error) stringToUTF8(String(error), pointer, size + 1);
      else if (result) HEAPU8.set(result, pointer);
      try { {{{ makeDynCall('viiii', 'callback') }}}(owner, status, pointer, size); }
      finally { _free(pointer); }
    });
  },
});
