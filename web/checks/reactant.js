async (page, context) => {
  const check = await context.start(page, context);
  // The desktop shell has a 340 px navigation column and 36 px canvas padding.
  // Compare only Composition's badge row, excluding the focused action button.
  const badges = { x: 404, y: 280, width: 510, height: 70 };
  const initial = await check.capture('original-badge-order', badges);
  await check.click(486, 156);
  await check.expectImage('reordered-badges', initial, badges, 0.008);
  await check.click(486, 156);
  await check.expectImage('restored-badges', initial, badges, 0, 0.001);
  return check.result('Reorder the composed badges and restore their original order');
}
