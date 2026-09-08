async (page, context) => {
  const check = await context.start(page, context);
  // The title snapshot has an empty board; Enter starts the game in input.rs.
  const board = { x: 390, y: 250, width: 500, height: 370 };
  const title = await check.capture('title-board', board);
  await check.canvas.focus();
  await page.keyboard.press('Enter', { delay: 200 });
  await check.expectImage('populated-board', title, board, 0.04);
  return check.result('Start a chess game with Enter and verify pieces appear on the board');
}
