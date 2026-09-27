async (page, context) => {
  // Keep the desktop layout at 1x backing resolution on the software renderer.
  await page.addInitScript(() => {
    Object.defineProperty(window, 'devicePixelRatio', { get: () => 1 });
    window.heartsAudioContexts = [];
    const AudioContext = window.AudioContext;
    window.AudioContext = new Proxy(AudioContext, {
      construct(target, arguments_, newTarget) {
        const audio = Reflect.construct(target, arguments_, newTarget);
        window.heartsAudioContexts.push(audio);
        return audio;
      }
    });
  });
  const check = await context.start(page, context);
  const completeHand = context.completeHand === true;
  const humanTurns = completeHand ? 13 : 2;
  const moves = [];
  const frames = [];
  const observeFrame = message => {
    if (!message.text().includes('battlement.frame.slow')) return;
    const duration = /battlement_duration_ms=([\d.]+)/.exec(message.text());
    if (duration) frames.push(Number(duration[1]));
  };
  page.on('console', observeFrame);
  // A short mouse pulse can fall between Unity frames under software rendering.
  const click = (x, y) => check.click(x, y, 800);
  const read = async filename => page.evaluate(async filename => {
    const db = await new Promise((resolve, reject) => {
      const request = indexedDB.open('/idbfs');
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    try {
      if (!db.objectStoreNames.contains('FILE_DATA')) return null;
      return await new Promise((resolve, reject) => {
        const transaction = db.transaction('FILE_DATA');
        const store = transaction.objectStore('FILE_DATA');
        const keys = store.getAllKeys(), values = store.getAll();
        transaction.oncomplete = () => {
          const index = keys.result.findIndex(key => String(key).endsWith('/' + filename));
          resolve(index < 0 ? null : new TextDecoder().decode(values.result[index].contents));
        };
        transaction.onabort = () => reject(transaction.error);
      });
    } finally { db.close(); }
  }, filename);
  const match = async () => JSON.parse(await read('hearts-match.json'));
  const waitMatch = async (description, predicate) => {
    const deadline = Date.now() + 30000;
    let saved;
    do {
      saved = await match();
      if (saved && predicate(saved.state)) return saved;
      await page.waitForTimeout(200);
    } while (Date.now() < deadline);
    throw new Error(`${description}: ${JSON.stringify(saved)}`);
  };
  const waitBright = async (name, clip, minimum, levels = [180, 160, 110]) => {
    const deadline = Date.now() + 30000;
    do {
      const capture = await check.capture(name, clip);
      const fraction = await page.evaluate(async ([bytes, levels]) => {
        const image = new Image(); image.src = 'data:image/png;base64,' + bytes; await image.decode();
        const surface = document.createElement('canvas'); surface.width = image.width; surface.height = image.height;
        const context = surface.getContext('2d'); context.drawImage(image, 0, 0);
        const pixels = context.getImageData(0, 0, image.width, image.height).data;
        let bright = 0;
        for (let i = 0; i < pixels.length; i += 4) if (pixels[i] > levels[0] && pixels[i + 1] > levels[1] && pixels[i + 2] > levels[2]) bright++;
        return bright / (pixels.length / 4);
      }, [capture, levels]);
      if (fraction >= minimum) return;
      await page.waitForTimeout(200);
    } while (Date.now() < deadline);
    throw new Error(`${name} did not render`);
  };
  const homeReady = () => waitBright('home-ready', { x: 580, y: 150, width: 120, height: 30 }, 0.5);
  // This input driver chooses one legal card; the production reducer alone admits moves.
  const choice = state => {
    const hand = state.hands[0];
    let candidates = hand.map((card, index) => ({ card, index }));
    const lead = state.trick[0]?.card.suit;
    if (lead && hand.some(card => card.suit === lead)) {
      candidates = candidates.filter(({ card }) => card.suit === lead);
    }
    if (!lead && !state.hearts_broken && hand.some(card => card.suit !== 'Hearts')) {
      candidates = candidates.filter(({ card }) => card.suit !== 'Hearts');
    }
    if (state.history.length < 4) {
      if (!lead) candidates = candidates.filter(({ card }) => card.suit === 'Clubs' && card.rank === 'Two');
      const safe = candidates.filter(({ card }) => card.suit !== 'Hearts' && !(card.suit === 'Spades' && card.rank === 'Queen'));
      if (safe.length) candidates = safe;
    }
    if (!candidates.length) throw new Error('The saved turn has no legal input');
    return candidates[0];
  };
  try {
    await homeReady();
    await click(640, 166); // New game.
    await page.keyboard.press('Tab', { delay: 800 });
    await page.keyboard.press('Enter', { delay: 800 }); // Confirm replacement through keyboard focus.
    await waitMatch('Initial durable deal', state => state.phase === 'Passing');
    const initialSave = await read('hearts-match.json');
    await waitBright('dealt-cards-ready', { x: 450, y: 520, width: 380, height: 50 }, 0.4, [80, 80, 80]);
    await click(640, 268); // Passing help.
    await waitMatch('Opponent passes', state => state.passes.slice(1).every(Boolean));
    await page.waitForTimeout(2000);
    await check.capture('dealt-hand');
    // Separated exposed regions avoid the overlap between neighboring rotated cards.
    for (const x of [400, 640, 950]) await click(x, 550);
    await click(650, 684);
    await waitMatch('Pass exchange', state => Boolean(state.phase?.Playing));
    await waitBright('play-help-ready', { x: 580, y: 280, width: 120, height: 30 }, 0.5);
    await click(640, 292); // Play help.
    await page.waitForFunction(() => window.heartsAudioContexts.some(audio => audio.state === 'running'));
    const audio = await page.evaluate(() => window.heartsAudioContexts.map(audio => audio.state));
    let resumed = false;
    for (let move = 0; move < humanTurns; move++) {
      const saved = await waitMatch('Human turn', state => state.phase?.Playing?.turn === 'South');
      const state = saved.state, hand = state.hands[0];
      if (move === (completeHand ? 4 : 1)) {
        const durable = await read('hearts-match.json');
        const connected = page.waitForEvent('console', { predicate: message => message.text().includes('battlement.host.connected'), timeout: 90000 });
        await page.reload();
        await connected;
        await homeReady();
        if (await read('hearts-match.json') !== durable) throw new Error('Reload changed the durable match');
        await click(640, 166); // Continue.
        await waitBright('hand-ready', { x: 450, y: 520, width: 380, height: 50 }, 0.5);
        await check.capture('resumed-hand');
        resumed = true;
      }
      const { card, index } = choice(state);
      await page.waitForTimeout(1000);
      const x = 640 + (index - (hand.length - 1) / 2 - 0.45) * 1.16 * 720 / 11.4;
      await click(x, 550);
      await click(800, 684); // Play selected card.
      const next = await waitMatch('Accepted human card', current => current.hands[0].length < hand.length);
      const human = next.state.history.filter(play => play.seat === 'South');
      if (human.length !== move + 1) throw new Error('Input did not admit exactly one human play');
      moves.push({ card: human.at(-1).card, target: card, revision: next.revision, history: next.state.history.length });
    }
    const final = await waitMatch('Continued game', state => completeHand
      ? state.phase === 'HandOver' : state.phase?.Playing?.turn === 'South');
    const state = final.state;
    if (!resumed) throw new Error('Missing mid-hand resume');
    if (completeHand) {
      const unique = new Set(state.history.map(play => JSON.stringify(play.card)));
      if (state.history.length !== 52 || unique.size !== 52 || state.hands.some(hand => hand.length)) {
        throw new Error('The completed durable hand does not account for all 52 cards');
      }
      if (!state.result) throw new Error('Missing hand score');
      await waitBright('results-ready', { x: 450, y: 60, width: 380, height: 50 }, 0.5);
      await check.capture('completed-hand');
    } else if (state.hands[0].length !== 13 - humanTurns) {
      throw new Error('The resumed hand did not retain its human plays');
    }
    const graphics = await page.evaluate(() => {
      const canvas = document.querySelector('#unity-canvas');
      const gl = canvas.getContext('webgl2');
      const extension = gl.getExtension('WEBGL_debug_renderer_info');
      return { width: canvas.width, height: canvas.height, density: devicePixelRatio,
        renderer: extension && gl.getParameter(extension.UNMASKED_RENDERER_WEBGL) };
    });
    const result = check.result(`Pass with pointer and keyboard, play ${humanTurns} human turns, and resume after reload`);
    return { ...result, assertions: result.assertions + 8, moves, resumed, completeHand, graphics, audio, initialSave,
      hand: { revision: final.revision, played: state.history.length, result: state.result },
      gameFrameMs: { count: frames.length, maximum: Math.max(0, ...frames) } };
  } finally { page.off('console', observeFrame); }
}
