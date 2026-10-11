const { defineConfig } = require("@playwright/test");
const linux = process.platform === "linux";

module.exports = defineConfig({
  testDir: "./web/tests",
  // Give software WebGPU the runner resources without competing browser processes.
  workers: process.env.CI ? 1 : undefined,
  snapshotPathTemplate: "{testDir}/snapshots/{arg}{ext}",
  use: {
    baseURL: "http://127.0.0.1:18080",
    browserName: "chromium",
    // Linux WebGPU screenshots require a display; CI supplies Xvfb.
    headless: !linux,
    launchOptions: {
      args: [
        "--enable-unsafe-webgpu",
        "--use-webgpu-adapter=swiftshader",
        ...(linux ? [
          "--enable-features=Vulkan",
          "--use-angle=vulkan",
          "--use-vulkan=swiftshader",
          "--disable-vulkan-surface",
        ] : ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"]),
      ],
    },
  },
  webServer: {
    command: "npm run serve:web",
    url: "http://127.0.0.1:18080",
    reuseExistingServer: !process.env.CI,
  },
});
