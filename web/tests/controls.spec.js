const path = require("node:path");
const { test, expect } = require("@playwright/test");
const { freezeClock } = require("./frozen-clock");
const screenshotOptions = { maxDiffPixels: 0, stylePath: path.join(__dirname, "screenshot.css") };
// The middle of the 960×540 canvas, away from the debug labels and the focus outline.
const cubeArea = { x: 280, y: 120, width: 400, height: 300 };

// Opens the demo with time frozen at 0 and checks it starts at the default camera.
async function start(page) {
  await freezeClock(page);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.setViewportSize({ width: 960, height: 540 });
  await page.goto("/");
  const canvas = page.getByLabel("reng game canvas");
  await expect(page.locator("#status")).toHaveCount(0);
  await expect(canvas).toHaveScreenshot("cube.png", screenshotOptions);
  return { canvas, errors, still: await page.screenshot({ clip: cubeArea }) };
}

// Waits until input reaches the screen, so a later screenshot never shows an older frame.
async function expectChangedFrom(page, before) {
  await expect.poll(async () => (await page.screenshot({ clip: cubeArea })).equals(before)).toBe(false);
}

async function expectSameAs(page, expected) {
  await expect.poll(async () => (await page.screenshot({ clip: cubeArea })).equals(expected)).toBe(true);
}

// Scrolls in until the camera stops at its closest distance, captures that view, and resets.
// Any zoom that reaches the same distance must match it exactly; a turn or a zoom out cannot.
async function closestView(page, canvas, still) {
  await canvas.focus();
  await page.mouse.move(480, 270);
  await page.mouse.wheel(0, -1000);
  await expectChangedFrom(page, still);
  const closest = await page.screenshot({ clip: cubeArea });
  await page.keyboard.press("Space");
  await expect(canvas).toHaveScreenshot("cube.png", screenshotOptions);
  return closest;
}

test("mouse drag and the scroll wheel move the camera, and Space resets it", async ({ page }) => {
  const { canvas, errors, still } = await start(page);

  await page.mouse.move(480, 270);
  await page.mouse.down();
  await page.mouse.move(600, 310, { steps: 4 });
  await page.mouse.up();
  await expectChangedFrom(page, still);
  // The click focused the canvas, so it receives keys.
  await page.keyboard.press("Space");
  await expect(canvas).toHaveScreenshot("cube.png", screenshotOptions);

  const closest = await closestView(page, canvas, still);
  // Browsers send a trackpad pinch as a Ctrl scroll of -100 × ln(scale) pixels; a 2.5x pinch
  // reaches the closest distance, where a plain scroll of the same size would barely move.
  const cdp = await page.context().newCDPSession(page);
  const deltaY = -100 * Math.log(2.5);
  await cdp.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: 480, y: 270, deltaX: 0, deltaY, modifiers: 2 });
  await expectSameAs(page, closest);
  await page.keyboard.press("Space");
  await expect(canvas).toHaveScreenshot("cube.png", screenshotOptions);
  expect(errors).toEqual([]);
});

test("a held W key moves the camera as time passes", async ({ page }) => {
  const { canvas, errors, still } = await start(page);
  await canvas.focus();
  await page.keyboard.down("KeyW");
  // Frozen time means no movement until the clock advances; let one frame see the key first.
  await page.evaluate(() => new Promise(requestAnimationFrame));
  await page.evaluate(() => window.rengSetTime(100));
  await expectChangedFrom(page, still);
  await page.keyboard.up("KeyW");
  const moved = await page.screenshot({ clip: cubeArea });

  // The cube's spin stays at 100 ms, so only the camera reset can change the picture now.
  await page.keyboard.press("Space");
  await expectChangedFrom(page, moved);
  expect(errors).toEqual([]);
});

test.describe("on a touch screen", () => {
  test.use({ hasTouch: true });

  test("one finger turns the camera and two fingers zoom", async ({ page }) => {
    const { canvas, errors, still } = await start(page);
    // Playwright's touchscreen only taps, so drive multi-touch through the DevTools protocol.
    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 2 });
    const touch = (type, touchPoints = []) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints });

    await touch("touchStart", [{ x: 480, y: 270, id: 0 }]);
    await touch("touchMove", [{ x: 600, y: 310, id: 0 }]);
    await touch("touchEnd");
    await expectChangedFrom(page, still);
    await canvas.focus();
    await page.keyboard.press("Space");
    await expect(canvas).toHaveScreenshot("cube.png", screenshotOptions);

    // Spreading the fingers from 80 to 200 pixels apart zooms in 2.5x, to the closest distance.
    const closest = await closestView(page, canvas, still);
    await touch("touchStart", [{ x: 440, y: 270, id: 0 }, { x: 520, y: 270, id: 1 }]);
    await touch("touchMove", [{ x: 380, y: 270, id: 0 }, { x: 580, y: 270, id: 1 }]);
    await touch("touchEnd");
    await expectSameAs(page, closest);
    await page.keyboard.press("Space");
    await expect(canvas).toHaveScreenshot("cube.png", screenshotOptions);
    expect(errors).toEqual([]);
  });
});
