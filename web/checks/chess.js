async (page, context) => {
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
  await page.mouse.click(320, 128, { delay: 200 });
  await waitForPlay(false);
  await check.expectImage('game-board', title, play, 0.2);
  return check.result('Wait for Play, click it, and verify the title menu leaves the board');
}
