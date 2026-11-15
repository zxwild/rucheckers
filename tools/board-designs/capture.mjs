#!/usr/bin/env node
/** Optional screenshot capture. Requires Playwright and a Chromium installation. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { mkdir, readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { designs, generate } from './generate.mjs';

const { chromium } = createRequire(import.meta.url)('playwright');
const site = resolve(process.argv[2] ?? 'work/board-designs');
const output = resolve(process.argv[3] ?? 'docs/board-designs');
await generate(site);
await mkdir(output, { recursive: true });
const server = createServer(async (request, response) => {
  // A small local preview server also works in browsers that restrict file://.
  const name = new URL(request.url, 'http://localhost').pathname.slice(1) || 'index.html';
  if (!/^(?:index|\d{2})\.html$/.test(name)) { response.writeHead(404).end(); return; }
  try { response.setHeader('Content-Type', 'text/html; charset=utf-8'); response.end(await readFile(resolve(site, name))); }
  catch { response.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;
let browser;
try {
  browser = await chromium.launch({ headless: true, ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}) });
  const page = await browser.newPage({ viewport: { width: 1680, height: 1100 }, deviceScaleFactor: 1 });
  const errors = [];
  const remoteRequests = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { if (!request.url().startsWith(base)) remoteRequests.push(request.url()); });
  await page.goto(`${base}/index.html`);
  const summary = await page.evaluate(() => {
    const ids = [...document.querySelectorAll('[id]')].map(e => e.id);
    return {
      boards: document.querySelectorAll('svg.board-svg').length,
      cells: document.querySelectorAll('[data-cell]').length,
      rasterImages: document.querySelectorAll('image,img,canvas').length,
      duplicateIds: ids.filter((id, i) => ids.indexOf(id) !== i),
    };
  });
  assert.equal(summary.boards, 12);
  assert.equal(summary.cells, 12 * 64);
  assert.equal(summary.rasterImages, 0);
  assert.deepEqual(summary.duplicateIds, []);
  await page.screenshot({ path: resolve(output, 'overview.png'), fullPage: true });
  await page.getByRole('button', { name: 'С шашками', exact: true }).click();
  assert.equal(await page.locator('.pieces:visible').count(), 12);
  await page.screenshot({ path: resolve(output, 'overview-with-pieces.png'), fullPage: true });
  await page.getByRole('button', { name: 'Без шашек', exact: true }).click();
  assert.equal(await page.locator('.pieces:visible').count(), 0);

  // Check small screens and real navigation, not just the static drawing.
  await page.setViewportSize({ width: 390, height: 844 });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await page.locator('a.card').first().click();
  assert.match(page.url(), /\/01\.html$/);
  await page.getByRole('link', { name: '← Все варианты' }).click();
  assert.match(page.url(), /\/index\.html$/);

  await page.setViewportSize({ width: 1280, height: 1500 });
  for (const d of designs) {
    await page.goto(`${base}/${d.id}.html`);
    const cells = await page.locator('[data-cell]').evaluateAll(nodes => nodes.map(n => ({ xy: n.getAttribute('data-cell'), tone: n.getAttribute('data-tone'), width: n.getAttribute('width'), height: n.getAttribute('height') })));
    assert.equal(cells.length, 64, d.id);
    assert.equal(new Set(cells.map(c => c.xy)).size, 64, d.id);
    for (const c of cells) {
      const [column, row] = c.xy.split(',').map(Number);
      assert.equal(c.tone, (column + row) % 2 ? 'dark' : 'light', `${d.id}: ${c.xy}`);
      assert.equal(c.width, c.height);
    }
    await page.locator('.card').screenshot({ path: resolve(output, `${d.id}.png`) });
    await page.getByRole('button', { name: 'С шашками', exact: true }).click();
    assert.equal(await page.locator('.pieces:visible').count(), 1);
    await page.locator('.card').screenshot({ path: resolve(site, `${d.id}-with-pieces.png`) });
    console.log(`${d.id}: 64 square cells, alternating colours, both views captured`);
  }
  assert.deepEqual(errors, []);
  assert.deepEqual(remoteRequests, []);
  console.log('Verified: 12 boards, unique SVG IDs, no raster inputs or external requests, view toggles, navigation, mobile layout.');
} finally {
  await browser?.close();
  await new Promise(resolve => server.close(resolve));
}
