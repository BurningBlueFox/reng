const { defineConfig } = require("@playwright/test");

module.exports = defineConfig({
  testDir: "./web/tests",
  snapshotPathTemplate: "{testDir}/snapshots/{arg}{ext}",
  use: {
    baseURL: "http://127.0.0.1:18080",
    browserName: "chromium",
    launchOptions: { args: ["--enable-unsafe-webgpu", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] },
  },
  webServer: {
    command: "npm run serve:web",
    url: "http://127.0.0.1:18080",
    reuseExistingServer: !process.env.CI,
  },
});
