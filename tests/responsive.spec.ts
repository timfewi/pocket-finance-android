import { test, expect, type Locator, type Page } from '@playwright/test';

const bridge = process.env.RECEIPT_BRIDGE_URL;
const navigation = (page: Page) => page.locator('nav[aria-label="Hauptnavigation"]:visible');

async function navigate(page: Page, name: 'Übersicht' | 'Erfassen' | 'Belege') {
  const button = navigation(page).getByRole('button', { name, exact: true });
  await button.click();
  await expect(button).toHaveAttribute('aria-current', 'page');
}

async function openReceipt(page: Page) {
  await page.getByRole('button', { name: 'Beispielbeleg prüfen', exact: true }).first().click();
  await expect(page.getByRole('heading', { name: /Einmal prüfen/ })).toBeVisible();
}

async function noHorizontalOverflow(page: Page) {
  const viewport = await page.evaluate(() => window.innerWidth);
  await expect.poll(
    () => page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth),
    { message: `Horizontal overflow at ${viewport}px viewport` }
  ).toBeLessThanOrEqual(1);
}

async function touchTarget(control: Locator) {
  const bounds = await control.boundingBox();
  expect(bounds, 'touch control must have a rendered box').not.toBeNull();
  expect(bounds!.width, 'touch target width').toBeGreaterThanOrEqual(47.5);
  expect(bounds!.height, 'touch target height').toBeGreaterThanOrEqual(47.5);
}

async function fitsViewport(control: Locator, page: Page) {
  const bounds = await control.boundingBox();
  expect(bounds).not.toBeNull();
  const width = await page.evaluate(() => window.innerWidth);
  expect(bounds!.x).toBeGreaterThanOrEqual(-1);
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(width + 1);
}

test.beforeEach(async ({ request, page }) => {
  if (bridge) {
    const reset = await request.post(`${bridge}/__test/reset`);
    expect(reset.ok()).toBeTruthy();
  }
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Alles an einem Ort.' })).toBeVisible();
  await expect(navigation(page)).toHaveCount(1);
  if (bridge) await expect(page.locator('.demo-notice')).toContainText('Rust-Testbrücke');
});

test('navigation preserves a reviewed draft and opens the saved receipt list', async ({ page }, info) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.screenshot({ path: `artifacts/mobile-first/${info.project.name}-navigation.png`, fullPage: true });
  await navigate(page, 'Erfassen');
  await expect(page.getByRole('heading', { name: 'Beleg erfassen', exact: true })).toBeVisible();
  await page.locator('.capture-choices').getByRole('button').first().click();
  await expect(page.getByRole('heading', { name: /Einmal prüfen/ })).toBeVisible();
  await page.getByLabel('Händler', { exact: true }).fill('Synthetischer Wochenmarkt');
  await page.getByLabel('Gesamtbetrag', { exact: true }).fill('18,75');
  await page.getByLabel('Kategorie', { exact: true }).selectOption('household');

  await navigate(page, 'Übersicht');
  await expect(page.getByRole('heading', { name: 'Alles an einem Ort.' })).toBeVisible();
  await expect(page.locator('.draft-resume')).toBeVisible();
  await page.locator('.draft-resume').getByRole('button', { name: /Entwurf weiter prüfen/ }).click();
  await expect(page.getByLabel('Händler', { exact: true })).toHaveValue('Synthetischer Wochenmarkt');
  await expect(page.getByLabel('Gesamtbetrag', { exact: true })).toHaveValue('18,75');
  await expect(page.getByLabel('Kategorie', { exact: true })).toHaveValue('household');

  await navigate(page, 'Belege');
  await expect(page.getByRole('heading', { name: 'Deine Belege', exact: true })).toBeVisible();
  await expect(page.locator('.transaction-row')).toHaveCount(0);
  await page.locator('.draft-resume').getByRole('button', { name: /Entwurf weiter prüfen/ }).click();
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: /Ausgabe speichern/ }).click();
  await expect(page.locator('#saved-message')).toBeVisible();
  await expect(navigation(page).getByRole('button', { name: 'Übersicht', exact: true })).toHaveAttribute('aria-current', 'page');
  await navigate(page, 'Belege');
  await expect(page.locator('.transaction-row')).toHaveCount(1);
  await expect(page.locator('.transaction-row')).toContainText('Synthetischer Wochenmarkt');
  await expect(page.locator('.transaction-row')).toContainText('18,75');
  await expect(page.locator('.draft-resume')).toHaveCount(0);
  await noHorizontalOverflow(page);
  expect(errors).toEqual([]);
});

test('phone, tablet and desktop widths contain long receipt text and large amounts', async ({ page }, info) => {
  test.skip(info.project.name !== 'mobile', 'Run the width matrix once; the navigation flow also runs on desktop.');
  const widths = [320, 360, 393, 412, 768, 1365];
  await openReceipt(page);
  await page.getByLabel('Händler', { exact: true }).fill(`SYNTHETISCH-${'Langbeleg'.repeat(14)}`);
  await page.getByLabel('Gesamtbetrag', { exact: true }).fill('9999999999,99');
  await page.getByLabel('Kategorie', { exact: true }).selectOption('groceries');
  for (const width of widths) {
    await page.setViewportSize({ width, height: 900 });
    await noHorizontalOverflow(page);
    await fitsViewport(page.getByLabel('Belegdatum'), page);
    await fitsViewport(page.getByLabel('Währung', { exact: true }), page);
    await fitsViewport(page.getByRole('button', { name: /Ausgabe speichern/ }), page);
    if (width === 393 || width === 1365) {
      await page.screenshot({ path: `artifacts/mobile-first/review-${width}.png`, fullPage: true });
    }
  }
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: /Ausgabe speichern/ }).click();
  await expect(page.locator('#saved-message')).toBeVisible();
  await expect(page.locator('.transaction-row')).toHaveCount(1);
  for (const width of widths) {
    await page.setViewportSize({ width, height: 900 });
    await noHorizontalOverflow(page);
    await fitsViewport(page.locator('.transaction-row'), page);
    await fitsViewport(page.locator('.transaction-amount'), page);
  }
});

test('narrow phone controls stay touchable and dialogs fit the viewport', async ({ page }, info) => {
  test.skip(info.project.name !== 'mobile', 'Mobile geometry check.');
  await page.setViewportSize({ width: 320, height: 740 });
  await expect(page.locator('.mobile-nav')).toBeVisible();
  for (const button of await navigation(page).getByRole('button').all()) await touchTarget(button);
  const nav = await page.locator('.mobile-nav').boundingBox();
  expect(nav).not.toBeNull();
  expect(nav!.y + nav!.height).toBeLessThanOrEqual(741);
  await navigate(page, 'Erfassen');
  for (const button of await page.locator('.capture-choices').getByRole('button').all()) await touchTarget(button);
  await page.locator('.capture-choices').getByRole('button').first().click();
  await expect(page.getByRole('heading', { name: /Einmal prüfen/ })).toBeVisible();
  for (const control of [page.getByLabel('Händler', { exact: true }), page.getByLabel('Belegdatum'), page.getByLabel('Gesamtbetrag', { exact: true }), page.getByLabel('Kategorie', { exact: true })]) {
    await touchTarget(control);
    expect(await control.evaluate(element => Number.parseFloat(getComputedStyle(element).fontSize))).toBeGreaterThanOrEqual(16);
  }
  await touchTarget(page.getByRole('button', { name: 'Beleg vergrößern', exact: true }));
  await page.getByRole('button', { name: 'Beleg vergrößern', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Originalbeleg', exact: true })).toBeVisible();
  await fitsViewport(page.getByRole('dialog', { name: 'Originalbeleg', exact: true }), page);
  const closePhoto = page.getByRole('button', { name: 'Vergrößerten Beleg schließen' });
  await touchTarget(closePhoto);
  await closePhoto.click();
  await touchTarget(page.getByRole('button', { name: /Ausgabe speichern/ }));
  const discard = page.getByRole('button', { name: 'Entwurf verwerfen', exact: true });
  await touchTarget(discard);
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  const lastControl = await discard.boundingBox();
  const bottomNav = await page.locator('.mobile-nav').boundingBox();
  expect(lastControl!.y + lastControl!.height).toBeLessThanOrEqual(bottomNav!.y);
  await discard.click();
  const dialog = page.getByRole('dialog', { name: 'Entwurf verwerfen?' });
  await expect(dialog).toBeVisible();
  await fitsViewport(dialog, page);
  for (const button of await dialog.getByRole('button').all()) await touchTarget(button);
  await page.screenshot({ path: 'artifacts/mobile-first/narrow-discard-dialog.png', fullPage: false });
  await dialog.getByRole('button', { name: 'Weiter prüfen' }).click();
  await noHorizontalOverflow(page);
});

test('focused amount remains accessible in a shortened viewport', async ({ page }, info) => {
  test.skip(info.project.name !== 'mobile', 'Viewport resize simulation, not an Android keyboard/device test.');
  await page.setViewportSize({ width: 393, height: 852 });
  await openReceipt(page);
  const amount = page.getByLabel('Gesamtbetrag', { exact: true });
  await amount.click();
  await page.setViewportSize({ width: 393, height: 440 });
  await expect(amount).toBeFocused();
  await expect.poll(async () => amount.evaluate(element => {
    const field = element.getBoundingClientRect();
    const bar = document.querySelector('.topbar')?.getBoundingClientRect();
    const nav = document.querySelector<HTMLElement>('.mobile-nav');
    const navVisible = nav && getComputedStyle(nav).display !== 'none' && nav.getBoundingClientRect().height > 0;
    const lowerEdge = navVisible ? nav!.getBoundingClientRect().top : window.innerHeight;
    return field.top >= Math.max(0, bar?.bottom ?? 0) && field.bottom <= lowerEdge;
  })).toBeTruthy();
  await amount.fill('42,80');
  await noHorizontalOverflow(page);
  await page.screenshot({ path: 'artifacts/mobile-first/short-viewport-focused-field.png', fullPage: false });
  await page.setViewportSize({ width: 393, height: 852 });
  await page.getByRole('heading', { name: /Einmal prüfen/ }).click();
  await expect(page.locator('.mobile-nav')).toBeVisible();
  await navigate(page, 'Übersicht');
  await page.locator('.draft-resume').getByRole('button', { name: /Entwurf weiter prüfen/ }).click();
  await expect(amount).toHaveValue('42,80');
});
