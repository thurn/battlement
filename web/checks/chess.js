async (page, context) => {
  await page.addInitScript(() => {
    window.chessAudioContexts = [];
    const AudioContext = window.AudioContext;
    window.AudioContext = new Proxy(AudioContext, {
      construct(target, arguments_, newTarget) {
        const audio = Reflect.construct(target, arguments_, newTarget);
        window.chessAudioContexts.push(audio);
        return audio;
      }
    });
  });
  const check = await context.start(page, { ...context, viewport: { width: 640, height: 360 } });
  // Host connection precedes menu rendering. Its neon Play row is absent from the board backdrop.
  const play = { x: 230, y: 110, width: 180, height: 36 };
  const waitForPlay = async visible => {
    const deadline = visible ? check.startupDeadline : Date.now() + 15000;
    do {
      const capture = await check.capture(visible ? 'play-ready' : 'play-dismissed', play);
      const neon = await page.evaluate(async value => {
        const image = new Image();
        image.src = 'data:image/png;base64,' + value;
        await image.decode();
        const surface = document.createElement('canvas');
        surface.width = image.width;
        surface.height = image.height;
        const pixels = surface.getContext('2d');
        pixels.drawImage(image, 0, 0);
        const data = pixels.getImageData(0, 0, image.width, image.height).data;
        let count = 0;
        for (let i = 0; i < data.length; i += 4) {
          if (data[i + 2] > 120 && data[i + 2] - data[i] > 40) count++;
        }
        return count / (data.length / 4);
      }, capture);
      if (visible ? neon > 0.05 : neon < 0.005) return capture;
      await page.waitForTimeout(200);
    } while (Date.now() < deadline);
    throw new Error(`Play row did not become ${visible ? 'visible' : 'dismissed'}`);
  };
  const title = await waitForPlay(true);
  async function click(x, y) {
    await page.mouse.move(x, y);
    await page.mouse.down();
    try {
      await page.evaluate(() => new Promise(resolve => {
        requestAnimationFrame(() => requestAnimationFrame(resolve));
      }));
    } finally { await page.mouse.up(); }
  }
  await click(320, 128);
  const board = await waitForPlay(false);
  const changed = await check.difference(title, board);
  if (changed < 0.2) throw new Error(`Game board changed fraction ${changed}, expected at least 0.2`);
  await page.waitForFunction(() => window.chessAudioContexts.some(audio => audio.state === 'running'));
  const audio = await page.evaluate(() => window.chessAudioContexts.map(audio => audio.state));
  const result = check.result('Activate audio and start a game through the visible Play menu');
  return { ...result, assertions: result.assertions + 2, audio, changed };
}
