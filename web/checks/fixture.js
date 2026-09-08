async (page, context) => {
  const runtime = await context.start(page, context);
  if (await page.evaluate(() => window.fixtureAssetDecoded) !== 'ok') {
    throw new Error('The browser did not decode and execute the compressed fixture asset');
  }
  const before = await runtime.capture('fixture-before', { x: 0, y: 0, width: 64, height: 64 });
  await page.evaluate(() => {
    const canvas = document.querySelector('#unity-canvas');
    const surface = canvas.getContext('2d');
    surface.fillStyle = '#def';
    surface.fillRect(0, 0, canvas.width, canvas.height);
  });
  await runtime.expectImage('fixture-after', before, { x: 0, y: 0, width: 64, height: 64 }, 0.9);
  return runtime.result('small browser harness fixture');
}
