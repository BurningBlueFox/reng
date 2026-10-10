const { test, expect } = require("@playwright/test");

test("starts one canvas, records spans, and handles viewport resizing", async ({ page }) => {
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
  await expect(canvas).toHaveCount(1);
  await expect(page.locator("#status")).toHaveCount(0);
  await expect.poll(() => canvas.boundingBox()).toMatchObject({ width: 960, height: 540 });
  await expect.poll(() => logs.join("\n")).toContain("Window created");
  await expect.poll(() => page.evaluate(() =>
    performance.getEntriesByType("measure").some((entry) => entry.name.startsWith("create_window "))
  )).toBe(true);

  const before = logs.filter((line) => line.includes("Window resized")).length;
  await page.setViewportSize({ width: 640, height: 360 });
  await expect.poll(() => canvas.boundingBox()).toMatchObject({ width: 640, height: 360 });
  await expect.poll(() => logs.filter((line) => line.includes("Window resized")).length).toBeGreaterThan(before);
  await expect.poll(() => logs.join("\n")).toContain("width=640 height=360");
  expect(errors).toEqual([]);
});

test("shows a startup error when the WebAssembly module cannot load", async ({ page }) => {
  await page.route("**/pkg/reng.js", (route) => route.abort());
  await page.goto("/");
  await expect(page.getByRole("alert")).toContainText("reng failed to start");
  await expect(page.locator("canvas")).toHaveCount(0);
});
