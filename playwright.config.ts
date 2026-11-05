import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  workers: 1,
  timeout: 30_000,
  outputDir: 'artifacts/playwright',
  reporter: [['list']],
  use: { baseURL: 'http://127.0.0.1:1420', locale: 'de-AT', timezoneId: 'Europe/Vienna', screenshot: 'only-on-failure', trace: 'retain-on-failure' },
  webServer: {
    command: 'npm run dev',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: !process.env.CI,
    env: { VITE_RECEIPT_BRIDGE_URL: process.env.RECEIPT_BRIDGE_URL ?? '' }
  },
  projects: [
    { name: 'desktop', use: { browserName: 'chromium', viewport: { width: 1365, height: 1000 } } },
    { name: 'mobile', use: { ...devices['Pixel 7'], defaultBrowserType: 'chromium' } }
  ]
});
