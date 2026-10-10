const { defineConfig } = require("@playwright/test");

module.exports = defineConfig({
  testDir: "./web/tests",
  use: { baseURL: "http://127.0.0.1:18080", browserName: "chromium" },
  webServer: {
    command: "npm run serve:web",
    url: "http://127.0.0.1:18080",
    reuseExistingServer: !process.env.CI,
  },
});
