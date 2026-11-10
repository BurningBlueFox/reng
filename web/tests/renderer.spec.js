const path = require("node:path");
const { test, expect } = require("@playwright/test");
const screenshotOptions = { maxDiffPixels: 0, stylePath: path.join(__dirname, "screenshot.css") };
// The middle of the 960×540 canvas, away from the top-right debug labels.
const cubeArea = { x: 280, y: 120, width: 400, height: 300 };

test("draws a spinning cube and redraws after resize and zero size", async ({ page }) => {
  // web-time, the engine clock, calls performance.now() on every Instant::now() through a dynamic
  // method lookup, so replacing it freezes the animation and labels without test hooks in
  // production code. page.clock would also fake requestAnimationFrame, which drives winit redraws.
  await page.addInitScript(() => {
    let now = 0;
    performance.now = () => now;
    window.rengSetTime = (milliseconds) => { now = milliseconds; };
  });
  const errors = [];
  const logs = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
    logs.push(message.text());
  });
  await page.setViewportSize({ width: 960, height: 540 });
  await page.goto("/");
  const canvas = page.getByLabel("reng game canvas");
  await expect.poll(() => logs.join("\n")).toContain("Renderer initialized");
  await expect(canvas).toHaveScreenshot("cube.png", screenshotOptions);

  // Advancing the frozen clock must turn the cube, whatever the golden images show.
  const still = await page.screenshot({ clip: cubeArea });
  await page.evaluate(() => window.rengSetTime(1000));
  await expect.poll(async () => (await page.screenshot({ clip: cubeArea })).equals(still)).toBe(false);

  // 4:3 rather than 16:9, so the golden image shows aspect correction.
  await page.setViewportSize({ width: 640, height: 480 });
  await expect(canvas).toHaveAttribute("width", "640");
  await expect(canvas).toHaveAttribute("height", "480");
  await expect(canvas).toHaveScreenshot("cube-resized.png", screenshotOptions);

  await canvas.evaluate((element) => { element.style.width = "0px"; element.style.height = "0px"; });
  await expect.poll(() => logs.join("\n")).toContain("width=0 height=0");
  await canvas.evaluate((element) => { element.style.width = "100%"; element.style.height = "100%"; });
  await expect(canvas).toHaveAttribute("width", "640");
  await expect(canvas).toHaveScreenshot("cube-resized.png", screenshotOptions);
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
