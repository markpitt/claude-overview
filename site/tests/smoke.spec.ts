import { test, expect, type Page } from '@playwright/test';

const BASE = '/claude-overview/';
const PAGES = [
  { path: '', h1: 'Claude Overview' },
  { path: 'install/', h1: 'Install' },
  { path: 'using-it/', h1: 'Using it' },
  { path: 'settings/', h1: 'Settings' },
  { path: 'updates/', h1: 'Updates and releases' },
  { path: 'privacy/', h1: 'Privacy' },
];

async function visit(page: Page, path: string) {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  const response = await page.goto(BASE + path);
  expect(response?.status()).toBe(200);
  await page.evaluate(() => document.fonts.ready);
  return errors;
}

for (const p of PAGES) {
  for (const scheme of ['dark', 'light'] as const) {
    test(`${p.path || 'home'} renders in the ${scheme} theme`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      const errors = await visit(page, p.path);
      await expect(page.locator('h1')).toHaveText(p.h1);
      await expect(page.locator('nav[aria-label="Site"]')).toBeVisible();
      await expect(page.locator('footer')).toContainText('MIT');
      // The body follows the scheme: no stored choice means system.
      const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
      expect(bg).toBe(scheme === 'dark' ? 'rgb(9, 9, 11)' : 'rgb(255, 251, 248)');
      // Nothing wider than the viewport, at phone width as well as desktop.
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(overflow).toBeLessThanOrEqual(0);
      expect(errors).toEqual([]);
    });
  }
}

test('every image on every page is served', async ({ page, request }) => {
  // Fetched rather than waited for: the screenshots are lazy, and one of each pair is hidden.
  for (const p of PAGES) {
    await visit(page, p.path);
    const srcs = await page.$$eval('img', (imgs) => imgs.map((i) => (i as HTMLImageElement).src));
    expect(srcs.length, p.path).toBeGreaterThanOrEqual(p.path === 'install/' || p.path === 'privacy/' ? 0 : 1);
    for (const src of srcs) {
      const res = await request.get(src);
      expect(res.status(), src).toBe(200);
      expect(res.headers()['content-type'], src).toMatch(/^image\//);
    }
  }
});

test('every link within the site resolves', async ({ page, request }) => {
  const seen = new Set<string>();
  for (const p of PAGES) {
    await visit(page, p.path);
    const hrefs = await page.$$eval('a[href]', (as) => as.map((a) => (a as HTMLAnchorElement).href));
    for (const href of hrefs) {
      const url = new URL(href);
      if (url.origin !== new URL(page.url()).origin) continue;
      url.hash = '';
      seen.add(url.pathname);
    }
  }
  for (const path of seen) {
    expect((await request.get(path)).status(), path).toBe(200);
  }
  expect(seen.size).toBeGreaterThanOrEqual(PAGES.length);
});

test('the sub-bar marks the current page', async ({ page }) => {
  await visit(page, 'privacy/');
  await expect(page.locator('.mpc-sub a[aria-current="page"]')).toHaveText('Privacy');
  await visit(page, 'settings/');
  await expect(page.locator('.mpc-sub a[aria-current="page"]')).toHaveText('Docs');
});

test('the header links back to markpitt.dev, with Projects current', async ({ page }) => {
  await visit(page, '');
  await expect(page.locator('.mpc-lockup')).toHaveAttribute('href', 'https://markpitt.dev/');
  const projects = page.locator('.nav-menu-links a[aria-current="page"]');
  await expect(projects).toHaveText('Projects');
  await expect(projects).toHaveAttribute('href', 'https://markpitt.dev/projects/');
});

test('the theme control cycles light, dark, system and is remembered', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await visit(page, '');
  const toggle = page.locator('[data-theme-toggle]');
  await expect(toggle).toHaveAttribute('data-theme-state', 'system');
  await toggle.click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(page.locator('[data-theme-toggle]')).toHaveAttribute('aria-label', 'Theme: Light. Press for Dark.');
  await page.locator('[data-theme-toggle]').click();
  await page.locator('[data-theme-toggle]').click();
  await expect(page.locator('html')).not.toHaveAttribute('data-theme', /.*/);
});

test('the screenshot shown matches the theme', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await visit(page, '');
  await expect(page.locator('.co-shot-light').first()).toBeVisible();
  await expect(page.locator('.co-shot-dark').first()).toBeHidden();
});
