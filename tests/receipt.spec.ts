import { test, expect, type Page } from '@playwright/test';
const bridge = process.env.RECEIPT_BRIDGE_URL;

async function openReceipt(page: Page) {
  await page.getByRole('button', { name: 'Beispielbeleg prüfen', exact: true }).first().click();
  await expect(page.getByRole('heading', { name: /Einmal prüfen/ })).toBeVisible();
}
async function confirmFields(page: Page) {
  await page.getByLabel('Kategorie', { exact: true }).selectOption('groceries');
  await page.getByRole('checkbox').check();
}

test.beforeEach(async ({ request, page }) => {
  if (bridge) { const reset = await request.post(`${bridge}/__test/reset`); expect(reset.ok()).toBeTruthy(); }
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Alles an einem Ort.' })).toBeVisible();
  await expect(page.locator('.demo-notice')).toContainText('Nur synthetische Belege.');
  if (bridge) await expect(page.locator('.demo-notice')).toContainText('Rust-Testbrücke');
});

test('cash receipt uses 1249 cents, missing model is nonblocking, duplicate save stays single', async ({ page, request }, info) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  let savedCommand: unknown;
  page.on('request', request => {
    if (request.url().endsWith('/invoke') && request.method() === 'POST') {
      const data = request.postDataJSON();
      if (data.command === 'save_receipt') savedCommand = data;
    }
  });
  await page.screenshot({ path: `artifacts/screenshots/${info.project.name}-overview.png`, fullPage: true });
  await openReceipt(page);
  await expect(page.getByLabel('Gesamtbetrag', { exact: true })).toHaveValue('12,49');
  await expect(page.getByRole('button', { name: /Ausgabe speichern/ })).toBeDisabled();
  await expect(page.locator('.category-helper')).toContainText('Wähle die Kategorie');
  await expect(page.locator('.receipt-preview img')).toBeVisible();
  await expect.poll(() => page.locator('.receipt-preview img').evaluate(img => (img as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await page.screenshot({ path: `artifacts/screenshots/${info.project.name}-review.png`, fullPage: true });
  await confirmFields(page);
  const save = page.getByRole('button', { name: /Ausgabe speichern/ });
  await expect(save).toBeEnabled();
  await save.evaluate(button => { (button as HTMLButtonElement).click(); (button as HTMLButtonElement).click(); });
  await expect(page.locator('#saved-message')).toBeVisible();
  await expect(page.locator('.transaction-row')).toHaveCount(1);
  await page.locator('.transaction-row').click();
  await expect(page.locator('.transaction-detail')).toContainText('1249 Cent');
  if (bridge) {
    expect(savedCommand).toBeTruthy();
    const repeated = await request.post(`${bridge}/invoke`, { data: savedCommand });
    expect(repeated.ok()).toBeTruthy();
    const list = await request.post(`${bridge}/invoke`, { data: { command: 'list_transactions', args: {} } });
    const records = await list.json();
    expect(records).toHaveLength(1);
    expect(records[0].totalCents).toBe(1249);
  }
  await page.screenshot({ path: `artifacts/screenshots/${info.project.name}-saved.png`, fullPage: true });
  expect(errors).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
});

test('ambiguous total stays blank until the user chooses', async ({ page }, info) => {
  await page.getByText('Testszenario auswählen', { exact: false }).click();
  await page.getByLabel('Synthetischer Beleg für den nächsten Durchlauf').selectOption('ambiguous');
  await openReceipt(page);
  await expect(page.getByLabel('Gesamtbetrag', { exact: true })).toHaveValue('');
  await confirmFields(page);
  await expect(page.getByRole('button', { name: /Ausgabe speichern/ })).toBeDisabled();
  await page.getByRole('button', { name: /14,49/ }).click();
  await expect(page.getByRole('checkbox')).not.toBeChecked();
  await page.getByRole('checkbox').check();
  await page.screenshot({ path: `artifacts/screenshots/${info.project.name}-ambiguous.png`, fullPage: true });
  await page.getByRole('button', { name: /Ausgabe speichern/ }).click();
  await expect(page.locator('#saved-message')).toBeVisible();
  await page.locator('.transaction-row').click();
  await expect(page.locator('.transaction-detail')).toContainText('1449 Cent');
});

test('OCR failure allows manual correction; discard needs confirmation', async ({ page }) => {
  await page.getByText('Testszenario auswählen', { exact: false }).click();
  await page.getByLabel('Synthetischer Beleg für den nächsten Durchlauf').selectOption('ocr_failure');
  await openReceipt(page);
  await expect(page.getByText('Der Text konnte nicht erkannt werden.', { exact: false })).toBeVisible();
  await page.getByLabel('Händler', { exact: true }).fill('Manuell geprüfter Testbeleg');
  await page.getByLabel('Belegdatum').fill('2026-11-05');
  await page.getByLabel('Gesamtbetrag', { exact: true }).fill('12,49');
  await confirmFields(page);
  await page.getByRole('button', { name: 'Entwurf verwerfen', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Entwurf verwerfen?' })).toBeVisible();
  await page.getByRole('button', { name: 'Weiter prüfen' }).click();
  await expect(page.getByLabel('Händler', { exact: true })).toHaveValue('Manuell geprüfter Testbeleg');
  await page.getByRole('button', { name: /Ausgabe speichern/ }).click();
  await expect(page.locator('#saved-message')).toBeVisible();
  await expect(page.locator('.transaction-row')).toHaveCount(1);
  await openReceipt(page);
  await page.getByRole('button', { name: 'Entwurf verwerfen', exact: true }).click();
  await page.getByRole('dialog', { name: 'Entwurf verwerfen?' }).getByRole('button', { name: 'Entwurf verwerfen', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Alles an einem Ort.' })).toBeVisible();
  await expect(page.locator('.transaction-row')).toHaveCount(1);
  await expect(page.getByRole('heading', { name: /Einmal prüfen/ })).toHaveCount(0);
});
