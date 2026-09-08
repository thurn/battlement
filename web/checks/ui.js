async (page, context) => {
  const check = await context.start(page, context);
  // The root has 20 px padding, a 300 px navigation column, and a 30 px brand.
  // The first two navigation entries select Components and Interactions.
  // This sample's ConstantPixelSize panel uses backing pixels, unlike Reactant.
  const scale = 1 / await page.evaluate(() => devicePixelRatio);
  const heading = { x: 365 * scale, y: 70 * scale, width: 700 * scale, height: 110 * scale };
  const components = await check.capture('components-heading', heading);
  await check.click(165 * scale, 122 * scale);
  await check.expectImage('interactions-heading', components, heading, 0.01);
  await check.click(165 * scale, 96 * scale);
  await check.expectImage('components-restored', components, heading, 0, 0.001);
  return check.result('Navigate from Components to Interactions and back');
}
