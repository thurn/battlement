async (page, context) => {
  const check = await context.start(page, context);
  // Project cube C at (2, 0, 0) through snapshot()'s perspective camera.
  // Keep the status text and asynchronously recolored cube B outside the crop.
  const region = { x: 718, y: 310, width: 116, height: 120 };
  const initial = await check.capture('initial-cube-c', region);
  await check.click(774, 367);
  await check.expectImage('cube-c-moved', initial, region, 0.2);
  // The 500 ms tween ends at (2, 0, 2), still under this projected point.
  await page.waitForTimeout(600);
  await check.click(754, 355);
  await check.expectImage('cube-c-restored', initial, region, 0, 0.01);
  return check.result('Click cube C to move it into the scene, then restore its original position');
}
