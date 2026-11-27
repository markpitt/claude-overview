import { defineConfig, devices } from '@playwright/test';

// Smoke tests against a real build, served by `astro preview` under the same base path the site
// ships with. SITE_PORT picks the port (default 4870).
const port = Number(process.env.SITE_PORT ?? 4870);

export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? 'github' : 'list',
  use: { baseURL: `http://localhost:${port}`, trace: 'off' },
  projects: [
    { name: 'desktop', use: { ...devices['Desktop Chrome'] } },
    { name: 'phone', use: { ...devices['Pixel 7'] } },
  ],
  webServer: {
    command: `npx astro build && npx astro preview --port ${port}`,
    url: `http://localhost:${port}/claude-overview/`,
    reuseExistingServer: false,
    timeout: 120_000,
    // Astro 7 backgrounds `astro preview` when it thinks an agent started it, which ends the
    // webServer early. Any non-empty value keeps it in the foreground.
    env: { ASTRO_PREVIEW_BACKGROUND: '1', ASTRO_TELEMETRY_DISABLED: '1' },
  },
});
