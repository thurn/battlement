async (page, context) => {
  const check = await context.start(page, context);
  // snapshot() uses an orthographic half-height of 5.6. Cell 0 is (-1.92, 1.22).
  const cell = { x: 468, y: 232, width: 95, height: 95 };
  const empty = await check.capture('empty-cell', cell);
  await check.click(517, 282);
  const occupied = await check.expectImage('player-mark', empty, cell, 0.04);
  await page.waitForTimeout(500);
  await check.click(517, 282);
  await check.expectImage('occupied-cell-unchanged', occupied, cell, 0, 0.001);
  return check.result('Place a mark in the top-left cell; clicking the occupied cell preserves it');
}
