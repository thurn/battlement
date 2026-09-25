const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const test = require('node:test');

function fixture(initial = [1]) {
  const path = '/idbfs/settings.json';
  const entry = (bytes, time = 10) => ({ mode: 1, timestamp: new Date(time), contents: Uint8Array.from(bytes) });
  const local = new Map(initial === null ? [] : [[path, entry(initial)]]);
  const remote = structuredClone(local);
  const transactions = [];
  const mount = { mountpoint: '/idbfs' };
  let denyRead = false;
  let denyPut = false;
  let denyLocalRead = false;
  let failWrite = false;
  const DB = {
    transaction(_stores, mode) {
      if (denyRead) throw new Error('Storage denied');
      const requests = [];
      const tx = {
        mode, requests, aborted: false,
        objectStore() {
          return {
            put(value, key) {
              if (denyPut) throw new Error('Quota exceeded');
              requests.push({ key, value: structuredClone(value) });
            },
            delete(key) { requests.push({ key }); },
            get(key) {
              const result = { result: structuredClone(remote.get(key)) };
              return result;
            },
          };
        },
        abort() { tx.aborted = true; },
      };
      transactions.push(tx);
      return tx;
    },
  };
  const IDBFS = {
    DB_STORE_NAME: 'FILE_DATA',
    getDB(_mount, done) { done(null, DB); },
    getLocalSet(_mount, done) { done(null, { entries: Object.fromEntries(local) }); },
    getRemoteSet(_mount, done) { done(null, { entries: Object.fromEntries(remote), db: DB }); },
    loadLocalEntry(key, done) {
      if (denyLocalRead) return done({ errno: 2 });
      done(local.has(key) ? null : { errno: 44 }, local.get(key));
    },
    storeLocalEntry(key, value, done) { local.set(key, structuredClone(value)); done(null); },
    syncfs(_mount, _populate, done) { done(null); },
    queuePersist(mount) { IDBFS.syncfs(mount, false, () => {}); },
  };
  mount.type = IDBFS;
  const FS = {
    lookupPath: () => ({ node: { mount } }),
    isFile: mode => mode === 1,
    analyzePath: key => ({ exists: local.has(key) }),
    writeFile(key, bytes) {
      local.set(key, entry(bytes));
      IDBFS.queuePersist(mount);
      if (failWrite) { failWrite = false; throw new Error('File write failed'); }
    },
    unlink(key) { local.delete(key); IDBFS.queuePersist(mount); },
    utime(key, _access, modified) { local.get(key).timestamp = modified; },
  };
  const library = {};
  const context = vm.createContext({
    LibraryManager: { library }, mergeInto: Object.assign, IDBFS, FS,
    PATH: { dirname: () => '/idbfs' }, Date: class extends Date { static now() { return 100; } },
  });
  const source = fs.readFileSync('Packages/com.battlement.client/Runtime/BattlementPersistence.jslib', 'utf8');
  vm.runInContext(source.replace(/\{\{\{.*?\}\}\}/g, '(function () {})'), context);
  context.BattlementPersistence = library.$BattlementPersistence;
  const bridge = context.BattlementPersistence;
  bridge.install();
  return {
    local, remote, transactions, path, bridge,
    replaceAutomatic() {
      IDBFS.queuePersist = mount => IDBFS.syncfs(mount, false, () => {});
    },
    denyRead(value = true) { denyRead = value; },
    denyPut(value = true) { denyPut = value; },
    denyLocalRead() { denyLocalRead = true; },
    failWrite() { failWrite = true; },
    start(kind, bytes = []) {
      const results = [];
      bridge.start(path, kind, Uint8Array.from(bytes), (error, result) => results.push({ error, result }));
      return results;
    },
    automatic() { IDBFS.queuePersist(mount); },
    settle(error = null) {
      const tx = transactions.shift();
      assert.ok(tx, 'expected a storage transaction');
      if (error || tx.aborted) {
        tx.error = error;
        tx.onabort();
      } else {
        if (tx.mode === 'readwrite') tx.requests.forEach(({ key, value }) => {
          if (value) remote.set(key, value);
          else remote.delete(key);
        });
        tx.oncomplete();
      }
    },
    bytes(map = remote) { return map.has(path) ? Array.from(map.get(path).contents) : null; },
  };
}

test('save and load acknowledge only a terminal transaction', () => {
  const f = fixture();
  const saved = f.start(1, [2]);
  assert.equal(saved.length, 0);
  assert.deepEqual(f.bytes(), [1]);
  f.settle();
  assert.equal(saved.length, 1);
  assert.equal(saved[0].error, null);
  const loaded = f.start(0);
  assert.equal(loaded.length, 0);
  f.settle();
  assert.deepEqual(Array.from(loaded[0].result), [2]);
});

test('automatic sync, rapid writes, and deletion share one ordered queue', () => {
  const f = fixture();
  const first = f.start(1, [2]);
  f.automatic();
  const second = f.start(1, [3]);
  const deleted = f.start(2);
  assert.equal(f.transactions.length, 1);
  f.settle();
  assert.equal(first.length, 1);
  assert.equal(second.length, 0);
  f.settle();
  assert.deepEqual(f.bytes(), [3]);
  assert.equal(second.length, 1);
  assert.equal(deleted.length, 0);
  f.settle();
  f.automatic();
  assert.equal(deleted.length, 1);
  assert.equal(f.bytes(), null);
  assert.equal(f.transactions.length, 0);
});

test('failed save restores the confirmed file before queued automatic sync', () => {
  const f = fixture();
  const result = f.start(1, [9]);
  f.automatic();
  f.settle(new Error('Quota exceeded'));
  assert.match(String(result[0].error), /Quota/);
  assert.deepEqual(f.bytes(), [1]);
  assert.deepEqual(f.bytes(f.local), [1]);
  assert.equal(f.transactions.length, 0);
  const retry = f.start(1, [8]);
  f.settle();
  assert.equal(retry[0].error, null);
  assert.deepEqual(f.bytes(), [8]);
});

test('failed deletion remains recoverable and a retry stays deleted', () => {
  const f = fixture();
  const result = f.start(2);
  f.automatic();
  f.settle(new Error('Deletion denied'));
  assert.match(String(result[0].error), /Deletion/);
  assert.deepEqual(f.bytes(), [1]);
  assert.deepEqual(f.bytes(f.local), [1]);
  f.start(2);
  f.settle();
  assert.equal(f.bytes(), null);
});

test('load denial is reported rather than mistaken for absence', () => {
  const f = fixture();
  f.denyRead();
  const result = f.start(0);
  assert.match(String(result[0].error), /denied/);
  f.denyRead(false);
  const retried = f.start(0);
  f.settle();
  assert.deepEqual(Array.from(retried[0].result), [1]);
});

test('synchronous put failure waits for abort and rolls back memory', () => {
  const f = fixture();
  f.denyPut();
  const result = f.start(1, [9]);
  assert.equal(result.length, 0);
  f.settle();
  assert.match(String(result[0].error), /Quota/);
  assert.deepEqual(f.bytes(f.local), [1]);
  assert.deepEqual(f.bytes(), [1]);
});

test('failed local preparation preserves prior bytes and releases the queue', () => {
  const f = fixture();
  f.failWrite();
  const result = f.start(1, [9]);
  assert.match(String(result[0].error), /write failed/);
  assert.deepEqual(f.bytes(f.local), [1]);
  f.denyLocalRead();
  assert.ok(f.start(2)[0].error);
  assert.deepEqual(f.bytes(f.local), [1]);
  assert.equal(f.bridge.busy, false);
});

test('failed initial save restores absence', () => {
  const f = fixture(null);
  const result = f.start(1, [9]);
  f.automatic();
  f.settle(new Error('Quota exceeded'));
  assert.ok(result[0].error);
  assert.equal(f.bytes(f.local), null);
  assert.equal(f.bytes(), null);
});

test('an older automatic transaction finishes before the next explicit write', () => {
  const f = fixture();
  f.local.get(f.path).contents = Uint8Array.from([4]);
  f.local.get(f.path).timestamp = new Date(20);
  f.automatic();
  const newer = f.start(1, [7]);
  assert.equal(newer.length, 0);
  assert.deepEqual(f.bytes(f.local), [4]);
  f.settle();
  assert.deepEqual(f.bytes(), [4]);
  f.settle();
  assert.deepEqual(f.bytes(), [7]);
  assert.equal(newer[0].error, null);
});


test('Unity startup can replace automatic persistence before the first request', () => {
  const f = fixture();
  f.replaceAutomatic();
  const result = f.start(1, [9]);
  assert.equal(f.bridge.queue.length, 0, 'explicit mutation must suppress automatic requests');
  f.settle(new Error('Quota exceeded'));
  assert.ok(result[0].error);
  assert.equal(f.bridge.queue.length, 0, 'rollback must not schedule another flush');
  assert.deepEqual(f.bytes(), [1]);
});
