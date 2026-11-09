const path = require("node:path");
const { test, expect } = require("@playwright/test");
// A low per-pixel threshold ignores one-level rasterization noise but catches a shifted gradient.
const screenshotOptions = { maxDiffPixels: 0, threshold: 0.01, stylePath: path.join(__dirname, "screenshot.css") };
// The gradient midpoint loops back to the unshifted gradient every 4 s (src/scene/data.rs).
const gradientPeriodMs = 4000;
// One fake animation frame; the first frame starts the animation clock at zero.
const frameMs = 16;

// Freezes page time so frames draw only when the test advances the clock.
async function openPausedPage(page) {
  const errors = [];
  const logs = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
    logs.push(message.text());
  });
  await page.clock.install({ time: 0 });
  await page.clock.pauseAt(1000);
  await page.setViewportSize({ width: 960, height: 540 });
  await page.goto("/");
  await expect.poll(() => logs.join("\n")).toContain("Renderer initialized");
  await page.clock.runFor(frameMs);
  await expect.poll(() => logs.join("\n")).toContain("Frame presented");
  return { canvas: page.getByLabel("reng game canvas"), errors, logs };
}

test("draws a shaded triangle and redraws after resize and zero size", async ({ page }) => {
  const { canvas, errors, logs } = await openPausedPage(page);
  await expect(canvas).toHaveScreenshot("triangle.png", screenshotOptions);

  const frames = logs.filter((line) => line.includes("Frame presented")).length;
  await page.setViewportSize({ width: 640, height: 360 });
  await expect(canvas).toHaveAttribute("width", "640");
  await expect(canvas).toHaveAttribute("height", "360");
  await page.clock.runFor(gradientPeriodMs);
  await expect.poll(() => logs.filter((line) => line.includes("Frame presented")).length).toBeGreaterThan(frames);
  await expect(canvas).toHaveScreenshot("triangle-resized.png", screenshotOptions);

  const beforeZero = logs.filter((line) => line.includes("Frame presented")).length;
  await canvas.evaluate((element) => { element.style.width = "0px"; element.style.height = "0px"; });
  await expect.poll(() => logs.join("\n")).toContain("width=0 height=0");
  await canvas.evaluate((element) => { element.style.width = "100%"; element.style.height = "100%"; });
  await expect(canvas).toHaveAttribute("width", "640");
  await page.clock.runFor(gradientPeriodMs);
  await expect.poll(() => logs.filter((line) => line.includes("Frame presented")).length).toBeGreaterThan(beforeZero);
  await expect(canvas).toHaveScreenshot("triangle-resized.png", screenshotOptions);
  expect(errors).toEqual([]);
});

test("animates the gradient midpoint and loops each period", async ({ page }) => {
  const { canvas, errors } = await openPausedPage(page);
  const start = await canvas.screenshot();
  await page.clock.runFor(gradientPeriodMs / 4);
  expect((await canvas.screenshot()).equals(start)).toBe(false);
  await page.clock.runFor(gradientPeriodMs * 3 / 4);
  await expect(canvas).toHaveScreenshot("triangle.png", screenshotOptions);
  expect(errors).toEqual([]);
});

test("reports unavailable WebGPU visibly", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "gpu", { value: undefined, configurable: true });
  });
  await page.goto("/");
  await expect(page.getByRole("alert")).toContainText("No suitable graphics adapter found");
  await expect(page.locator("canvas")).toHaveCount(0);
});

test("shows asynchronous GPU validation failures and stops drawing", async ({ page }) => {
  await page.addInitScript(() => {
    window.rengTestDevices = [];
    const requestDevice = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await requestDevice.apply(this, args);
      window.rengTestDevices.push(device);
      return device;
    };
  });
  await page.goto("/");
  await expect(page.locator("#status")).toHaveCount(0);
  await page.evaluate(() => {
    // wgpu can also create a temporary device to probe browser capabilities.
    const device = window.rengTestDevices.find((entry) => entry.onuncapturederror);
    device.createTexture({ size: [1, 1], format: "rgba8unorm", usage: 0 });
    device.queue.submit([]);
  });
  await expect(page.getByRole("alert")).toContainText("GPU error:");
  await expect(page.locator("canvas")).toHaveCount(0);
});
