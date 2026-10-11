const path = require("node:path");
const { test, expect } = require("@playwright/test");
const screenshotOptions = { maxDiffPixels: 0, stylePath: path.join(__dirname, "screenshot.css") };

test("clears the GPU canvas and redraws after resize and zero size", async ({ page }) => {
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
  await expect.poll(() => logs.join("\n")).toContain("Frame presented");
  await expect(canvas).toHaveScreenshot("background.png", screenshotOptions);

  const frames = logs.filter((line) => line.includes("Frame presented")).length;
  await page.setViewportSize({ width: 640, height: 360 });
  await expect(canvas).toHaveAttribute("width", "640");
  await expect(canvas).toHaveAttribute("height", "360");
  await expect.poll(() => logs.filter((line) => line.includes("Frame presented")).length).toBeGreaterThan(frames);
  await expect(canvas).toHaveScreenshot("background-resized.png", screenshotOptions);

  const beforeZero = logs.filter((line) => line.includes("Frame presented")).length;
  await canvas.evaluate((element) => { element.style.width = "0px"; element.style.height = "0px"; });
  await expect.poll(() => logs.join("\n")).toContain("width=0 height=0");
  await canvas.evaluate((element) => { element.style.width = "100%"; element.style.height = "100%"; });
  await expect(canvas).toHaveAttribute("width", "640");
  await expect.poll(() => logs.filter((line) => line.includes("Frame presented")).length).toBeGreaterThan(beforeZero);
  await expect(canvas).toHaveScreenshot("background-resized.png", screenshotOptions);
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
