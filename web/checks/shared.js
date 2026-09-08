async (page, context) => {
  const { url, evidencePrefix, waitForLog } = context;
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto(url);
  await waitForLog('battlement.host.connected');
  if (!await page.evaluate(() => crossOriginIsolated && typeof SharedArrayBuffer === 'function')) {
    throw new Error('The deployed player is not cross-origin isolated');
  }
  const canvas = page.locator('#unity-canvas');
  await canvas.waitFor({ state: 'visible' });
  let assertions = 1;
  const differences = [], captures = [];
  return {
    canvas,
    async click(x, y) {
      const box = await canvas.boundingBox();
      if (!box || x < 0 || y < 0 || x >= box.width || y >= box.height) throw new Error('Canvas input is outside the viewport');
      await page.mouse.click(box.x + x, box.y + y, { delay: 200 });
      await page.waitForTimeout(200);
      await page.mouse.move(0, 0);
    },
    async capture(name, clip) {
      const started = Date.now();
      const bytes = await page.screenshot({ path: `${evidencePrefix}-${name}.png`, clip, scale: 'css', timeout: 60000 });
      captures.push({ name, duration_ms: Date.now() - started });
      return bytes.toString('base64');
    },
    async difference(before, after) {
      return page.evaluate(async ([a, b]) => {
        const decode = async value => {
          const img = new Image();
          img.src = 'data:image/png;base64,' + value;
          await img.decode();
          const surface = document.createElement('canvas');
          surface.width = img.width;
          surface.height = img.height;
          const ctx = surface.getContext('2d');
          ctx.drawImage(img, 0, 0);
          return ctx.getImageData(0, 0, img.width, img.height).data;
        };
        const [left, right] = await Promise.all([decode(a), decode(b)]);
        if (left.length !== right.length) throw new Error('Compared regions have different sizes');
        let changed = 0;
        for (let i = 0; i < left.length; i += 4) {
          const delta = Math.max(...[0, 1, 2].map(c => Math.abs(left[i + c] - right[i + c])));
          if (delta > 24) changed++;
        }
        return changed / (left.length / 4);
      }, [before, after]);
    },
    async expectImage(name, before, clip, minimum, maximum = 1) {
      const deadline = Date.now() + 15000;
      let fraction;
      do {
        const after = await this.capture(name, clip);
        fraction = await this.difference(before, after);
        if (fraction >= minimum && fraction <= maximum) {
          assertions++;
          differences.push({ name, fraction, minimum, maximum });
          return after;
        }
        await page.waitForTimeout(200);
      } while (Date.now() < deadline);
      throw new Error(`${name}: changed fraction ${fraction}, expected ${minimum}..${maximum}`);
    },
    result(interaction) { return { interaction, assertions, differences, captures }; }
  };
}
