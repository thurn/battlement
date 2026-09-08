async (page, context) => {
  const check = await context.start(page, context);
  // PortraitViewport fits 1024x1536 at 0.75 of the desktop height fit.
  // MainMenu's Settings action is the second 140 px row at design y=640.
  const content = { x: 520, y: 265, width: 240, height: 180 };
  await page.waitForTimeout(1500);
  const menu = await check.capture('main-menu-actions', content);
  await check.click(640, 340);
  await check.expectImage('settings-panel', menu, content, 0.2);
  await page.waitForTimeout(1000);
  // ReturnButton is fixed at design (328, 1358), inside the 29 px frame inset.
  await check.click(650, 599);
  await check.expectImage('main-menu-restored', menu, content, 0, 0.04);
  return check.result('Open Settings and return to the main menu');
}
