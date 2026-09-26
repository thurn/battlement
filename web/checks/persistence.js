async (page, context) => {
  const cases = [];
  let assertions = 0;
  const equal = (actual, expected, reason) => {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      throw new Error(`${reason}: ${JSON.stringify(actual)} != ${JSON.stringify(expected)}`);
    }
    assertions++;
  };
  const load = async () => {
    await page.goto(context.url);
    await page.waitForFunction(() => !!window.fixture);
    await page.evaluate(() => {
      const f = window.fixture;
      f.events = [];
      f.fault = {};
      const transaction = IDBDatabase.prototype.transaction;
      IDBDatabase.prototype.transaction = function (...args) {
        if (f.fault.denied) throw new DOMException('Injected storage denial', 'SecurityError');
        const tx = transaction.apply(this, args);
        for (const event of ['complete', 'abort']) {
          tx.addEventListener(event, () => f.events.push({
            event, mode: tx.mode,
            status: f.current === undefined ? null : f.snapshot(f.current).status,
          }));
        }
        return tx;
      };
      const put = IDBObjectStore.prototype.put;
      IDBObjectStore.prototype.put = function (...args) {
        if (f.fault.quota) {
          f.fault.quota = false;
          throw new DOMException('Injected quota exhaustion', 'QuotaExceededError');
        }
        const tx = this.transaction;
        const request = put.apply(this, args);
        request.addEventListener('success', () => {
          f.events.push({ event: 'request-success', bytes: Array.from(args[0].contents || []) });
          if (f.fault.abort) {
            f.fault.abort = false;
            tx.abort();
          }
        });
        return request;
      };
      const start = f.bridge.start;
      f.bridge.start = function (path, kind, bytes, done) {
        start(path, kind, bytes, (error, result) => {
          if (kind === 1 && !error && f.fault.holdCompletion) {
            f.fault.holdCompletion = false;
            f.release = () => { f.release = null; done(error, result); };
          } else done(error, result);
        });
      };
      f.readDurable = () => new Promise((resolve, reject) => {
        f.idbfs.getDB('/idbfs', (error, db) => {
          if (error) return reject(error);
          const tx = db.transaction([f.idbfs.DB_STORE_NAME], 'readonly');
          const request = tx.objectStore(f.idbfs.DB_STORE_NAME).get('/idbfs/match.json');
          tx.onabort = () => reject(tx.error);
          tx.oncomplete = () => resolve(request.result
            ? JSON.parse(new TextDecoder().decode(request.result.contents)) : null);
        });
      });
    });
  };
  const reset = () => page.evaluate(() => {
    const f = window.fixture;
    f.current = f.reset();
    f.initial = f.snapshot(f.current);
    return f.current;
  });
  const snapshot = index => page.evaluate(index => window.fixture.snapshot(index), index);
  const status = async (index, expected) => {
    await page.waitForFunction(([index, expected]) => window.fixture.snapshot(index).status === expected, [index, expected]);
    return snapshot(index);
  };
  const update = (index, value) => page.evaluate(([index, value]) => window.fixture.update(index, value), [index, value]);
  const durable = () => page.evaluate(() => window.fixture.readDurable());
  await load();
  let owner = await reset();
  equal(await page.evaluate(() => window.fixture.initial.status), 'Loading', 'hydration must be asynchronous');
  await status(owner, 'Absent');
  await update(owner, 1);
  const first = await status(owner, 'Committed');
  equal(first.durable, 1, 'acknowledged value');
  equal(first.committed, first.revision, 'acknowledged immutable revision');
  equal(await page.evaluate(() => window.fixture.events.some(
    e => e.event === 'complete' && e.mode === 'readwrite' && e.status === 'Pending'
  )), true, 'Rust remains pending until transaction completion');
  await load();
  owner = await reset();
  const restored = await status(owner, 'Loaded');
  equal(restored.durable, 1, 'reload restores a complete record');
  equal(restored.committed, null, 'a read is not a new commit');
  equal(restored.owner === first.owner, false, 'reload starts a new owner');
  cases.push('asynchronous hydration, transaction acknowledgment, reload');

  await page.evaluate(() => { window.fixture.fault.denied = true; });
  owner = await reset();
  const denied = await status(owner, 'Error');
  equal(denied.error.includes('SecurityError'), true, 'denial is distinct from absence');
  await page.evaluate(() => { window.fixture.fault.denied = false; window.fixture.retry(window.fixture.current); });
  equal((await status(owner, 'Loaded')).durable, 1, 'denial retry restores prior bytes');
  await page.evaluate(() => {
    const f = window.fixture;
    f.idbfs.dbs['/idbfs'].close();
    delete f.idbfs.dbs['/idbfs'];
    const open = IDBFactory.prototype.open;
    IDBFactory.prototype.open = () => { throw new DOMException('Injected database denial', 'SecurityError'); };
    f.restoreOpen = () => { IDBFactory.prototype.open = open; };
  });
  owner = await reset();
  equal((await status(owner, 'Error')).error.includes('SecurityError'), true, 'database initialization denial is recoverable');
  await page.evaluate(() => { window.fixture.restoreOpen(); window.fixture.retry(window.fixture.current); });
  equal((await status(owner, 'Loaded')).durable, 1, 'initialization retry restores prior bytes');
  for (const value of [9, 8]) {
    await page.evaluate(() => { window.fixture.fault.quota = true; });
    await update(owner, value);
    const failed = await status(owner, 'Error');
    equal(failed.error.includes('QuotaExceededError'), true, 'quota failure is recoverable');
    equal(failed.durable, 1, 'failed write keeps last acknowledged value');
    equal(await durable(), 1, 'failed transaction preserves durable record');
  }
  await page.evaluate(() => window.fixture.retry(window.fixture.current));
  equal((await status(owner, 'Committed')).durable, 8, 'retry saves latest intent');
  cases.push('denial and quota failures preserve data; retry saves latest intent');

  await page.evaluate(() => { window.fixture.events.length = 0; window.fixture.fault.abort = true; });
  await update(owner, 11);
  await status(owner, 'Error');
  equal(await durable(), 8, 'aborted successful request cannot commit');
  equal(await page.evaluate(() => {
    const events = window.fixture.events.map(e => e.event);
    const success = events.indexOf('request-success');
    return success >= 0 && success < events.indexOf('abort');
  }), true, 'request succeeds before the injected real transaction abort');
  cases.push('request success is not transaction success');

  await page.evaluate(() => { window.fixture.events.length = 0; window.fixture.fault.holdCompletion = true; });
  await update(owner, 21);
  await page.waitForFunction(() => !!window.fixture.release);
  await update(owner, 22);
  await update(owner, 23);
  const old = owner;
  owner = await reset();
  await update(owner, 31);
  equal((await snapshot(owner)).status, 'Loading', 'replacement owner waits behind old physical write');
  equal(await durable(), 21, 'active request captured its immutable bytes');
  await page.evaluate(() => window.fixture.release());
  equal((await status(owner, 'Committed')).durable, 31, 'new owner wins after old acknowledgment');
  await update(old, 99);
  equal(await durable(), 31, 'retained old setter cannot overwrite new owner');
  equal(await page.evaluate(() => window.fixture.events
    .filter(e => e.event === 'request-success')
    .map(e => JSON.parse(new TextDecoder().decode(Uint8Array.from(e.bytes))))
  ), [21, 31], 'superseded queued values never reach IndexedDB');
  cases.push('immutable/coalesced writes and replacement-owner ordering');

  await page.evaluate(() => { window.fixture.bridge.flush = () => {}; });
  await update(owner, 41);
  equal((await snapshot(owner)).status, 'Pending', 'uncommitted update remains pending');
  await load();
  owner = await reset();
  equal((await status(owner, 'Loaded')).durable, 31, 'reload before commit retains old record');
  await page.evaluate(() => { window.fixture.fault.holdCompletion = true; });
  await update(owner, 51);
  await page.waitForFunction(() => !!window.fixture.release);
  equal((await snapshot(owner)).status, 'Pending', 'commit callback is still held');
  equal(await durable(), 51, 'complete commit precedes its acknowledgment');
  await load();
  owner = await reset();
  equal((await status(owner, 'Loaded')).durable, 51, 'reload accepts commit even without prior callback');
  cases.push('reload before commit and after commit before acknowledgment');

  await page.evaluate(() => {
    const f = window.fixture;
    const transaction = IDBDatabase.prototype.transaction;
    IDBDatabase.prototype.transaction = function (...args) {
      const tx = transaction.apply(this, args);
      if (tx.mode === 'readwrite') queueMicrotask(() => tx.abort());
      return tx;
    };
    f.restoreTransactions = () => { IDBDatabase.prototype.transaction = transaction; };
    f.clear(f.current);
  });
  await status(owner, 'Error');
  equal(await durable(), 51, 'failed deletion preserves record');
  await page.evaluate(() => { window.fixture.restoreTransactions(); window.fixture.retry(window.fixture.current); });
  equal((await status(owner, 'Committed')).durable, null, 'deletion retry commits absence');
  await load();
  owner = await reset();
  equal((await status(owner, 'Absent')).durable, null, 'deleted record remains absent after reload');
  cases.push('deletion abort and retry survive reload');
  return { interaction: 'Exercise typed Rust persistence against real IDBFS transactions and reloads', assertions, cases };
}
