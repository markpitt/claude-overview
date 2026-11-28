import { test, expect } from '@playwright/test';

// The 404 page, as markpitt.dev's (markpitt.github.io docs/specs/not-found.md), at this site's base.
const BASE = '/claude-overview/';

test('an unknown address gets a 404 status and the page, kept out of search', async ({ page }) => {
  const response = await page.goto(`${BASE}no/such/page/`);
  expect(response?.status()).toBe(404);
  await expect(page.locator('h1')).toHaveText('Page not found');
  await expect(page).toHaveTitle('Page not found · Claude Overview');
  await expect(page.locator('meta[name="robots"]')).toHaveAttribute('content', 'noindex');
  await expect(page.locator('link[rel="canonical"]')).toHaveCount(0);
  await expect(page.locator('.nf-code')).toHaveAttribute('aria-hidden', 'true');
});

test('the routes back stay under the base, at any depth', async ({ page }) => {
  await page.goto(`${BASE}a/b/c/`);
  const hrefs = await page.locator('.nf-routes a').evaluateAll((as) => as.map((a) => a.getAttribute('href')));
  expect(hrefs).toEqual([BASE, `${BASE}install/`, `${BASE}using-it/`, '/']);
  // Styles load from the base too, so the numerals are drawn, not left as plain text.
  const size = await page.locator('.nf-code').evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
  expect(size).toBeGreaterThanOrEqual(104);
});

test('the address is shown as text, and a typo offers the page it meant', async ({ page }) => {
  await page.goto(`${BASE}instal/`);
  await expect(page.locator('.nf-lede .nf-path')).toHaveText(`${BASE}instal/`);
  await expect(page.locator('[data-nf-term]')).toContainText(`curl -sI https://markpitt.dev${BASE}instal/`);
  await expect(page.locator('[data-nf-suggest] a')).toHaveAttribute('href', `${BASE}install/`);
  const report = await page.locator('[data-nf-report]').getAttribute('href');
  expect(report).toBe(`mailto:mark@markpitt.dev?subject=${encodeURIComponent(`Broken link on markpitt.dev: ${BASE}instal/`)}`);
});

test('nothing close, nothing suggested', async ({ page }) => {
  await page.goto(`${BASE}wp-admin/`);
  await expect(page.locator('[data-nf-suggest]')).toBeHidden();
});

test.describe('without JavaScript', () => {
  test.use({ javaScriptEnabled: false });
  test('the routes and the report link are still there', async ({ page }) => {
    await page.goto(`${BASE}instal/`);
    await expect(page.locator('.nf-routes a')).toHaveCount(4);
    await expect(page.locator('.nf-lede')).toContainText("There's nothing at this address.");
    await expect(page.locator('[data-nf-term]')).toBeHidden();
    await expect(page.locator('[data-nf-suggest]')).toBeHidden();
  });
});

for (const scheme of ['dark', 'light'] as const) {
  test(`fits the viewport and the mark keeps still for reduced motion, ${scheme}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: 'reduce' });
    await page.goto(`${BASE}a-really-long-address/that-goes/on/and-on/and-on-for-a-while/index.html`);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    const name = await page.locator('.nf-pupil').evaluate((el) => getComputedStyle(el).animationName);
    expect(name).toBe('none');
  });
}
