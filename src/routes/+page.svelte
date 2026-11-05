<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Camera, ImagePlus, ReceiptText, ArrowUpRight, ArrowLeft, Check, CheckCheck, ShieldCheck, WifiOff, Plus, ChevronRight, Expand, X, LoaderCircle, AlertCircle, Sparkles, Wallet, ScanLine, Leaf, CheckCircle2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import * as Card from '$lib/components/ui/card';
  import * as api from '$lib/api';
  import { editMoney, formatDate, formatMoney, inputCents } from '$lib/format';
  import type { Category, CategorySuggestion, DemoScenario, ReceiptDraft, Transaction } from '$lib/types';

  let native = $state(false);
  let connectedDemo = $state(false);
  let initialized = $state(false);
  let categories = $state<Category[]>([]);
  let transactions = $state<Transaction[]>([]);
  let draft = $state<ReceiptDraft | null>(null);
  let merchant = $state('');
  let receiptDate = $state('');
  let amount = $state('');
  let categoryId = $state('');
  let currency = $state('EUR');
  let notice = $state('');
  let confirmed = $state(false);
  let busy = $state<'capture' | 'ocr' | 'saving' | 'discarding' | null>(null);
  let error = $state('');
  let saved = $state<Transaction | null>(null);
  let scenario = $state<DemoScenario>('standard');
  let suggestion = $state<CategorySuggestion | null>(null);
  let suggesting = $state(false);
  let detail = $state<Transaction | null>(null);
  let photoDialog: HTMLDialogElement;
  let discardDialog: HTMLDialogElement;
  let actionNumber = 0;
  const amountCents = $derived(inputCents(amount));
  const photo = $derived(api.previewUrl(draft?.previewRef ?? null));
  const totalSpent = $derived(transactions.reduce((sum, item) => sum + item.totalCents, 0));
  const valid = $derived(merchant.trim().length > 0 && /^\d{4}-\d{2}-\d{2}$/.test(receiptDate) && amountCents !== null && categories.some(c => c.id === categoryId) && currency === 'EUR');
  const currencyUnclear = $derived(Boolean(draft?.warnings.some(w => /currency.*not printed|currency.*missing|missing_currency/i.test(w))));
  const ambiguous = $derived(Boolean(draft && draft.totalCents === null && draft.amountCandidates.length > 1));

  onMount(() => {
    native = api.nativeRuntime();
    connectedDemo = api.browserBridge();
    void load();
  });

  async function load() {
    error = '';
    try { [categories, transactions] = await Promise.all([api.listCategories(), api.listTransactions()]); }
    catch { error = 'Deine Übersicht konnte nicht geladen werden. Bitte versuche es erneut.'; }
    finally { initialized = true; }
  }

  async function start(source: 'camera' | 'import') {
    if (busy) return;
    const current = ++actionNumber;
    busy = 'capture'; error = ''; notice = ''; saved = null; detail = null;
    try {
      const image = source === 'camera' ? await api.captureReceipt(scenario) : await api.importReceipt(scenario);
      if (current !== actionNumber) return;
      busy = 'ocr';
      const next = await api.draftFromReceipt(image.imageRef);
      if (current !== actionNumber) return;
      draft = next;
      merchant = next.merchant ?? ''; receiptDate = next.date ?? ''; amount = editMoney(next.totalCents);
      categoryId = next.categoryId ?? ''; currency = next.currency; confirmed = false; suggestion = null;
      busy = null;
      await tick();
      document.getElementById('review-heading')?.focus();
      void getSuggestion(next.id);
    } catch (cause) {
      const code = typeof cause === 'string' ? cause : cause instanceof Error ? cause.message : cause && typeof cause === 'object' && 'code' in cause && typeof cause.code === 'string' ? cause.code : '';
      if (!/cancelled|canceled|abgebrochen/i.test(code)) error = 'Der Beleg konnte nicht geöffnet werden. Bitte versuche ein anderes Bild.';
    } finally { if (current === actionNumber) busy = null; }
  }

  async function getSuggestion(id: string) {
    suggesting = true;
    try {
      const result = await api.suggestCategory(id);
      if (draft?.id !== id) return;
      suggestion = result;
      if (result.status === 'suggested' && result.categoryId && categories.some(c => c.id === result.categoryId) && !categoryId && busy !== 'saving') {
        categoryId = result.categoryId; confirmed = false;
      }
    } catch { if (draft?.id === id) suggestion = { status: 'failed', categoryId: null }; }
    finally { if (draft?.id === id) suggesting = false; }
  }

  function changed() { confirmed = false; saved = null; }
  function choose(cents: number) { amount = editMoney(cents); changed(); }

  async function save() {
    if (!draft || busy || !valid || !confirmed || amountCents === null) return;
    busy = 'saving'; error = '';
    try {
      const result = await api.saveReceipt({ draftId: draft.id, merchant: merchant.trim(), date: receiptDate, totalCents: amountCents, currency, categoryId, confirmed: true });
      saved = result;
      transactions = [result, ...transactions.filter(t => t.id !== result.id)].slice(0, 200);
      draft = null; confirmed = false; suggesting = false;
      await tick(); document.getElementById('saved-message')?.focus();
    } catch { error = 'Speichern nicht möglich. Deine Eingaben bleiben erhalten. Bitte prüfe die Angaben und versuche es erneut.'; }
    finally { busy = null; }
  }

  function askDiscard() { if (draft && !busy) discardDialog.showModal(); }

  async function discard() {
    if (!draft || busy) return;
    discardDialog.close();
    busy = 'discarding'; error = '';
    try { const result = await api.discardReceipt(draft.id); draft = null; confirmed = false; suggesting = false; actionNumber++; if (result.imageCleanupPending) notice = 'Der Entwurf wurde verworfen. Das zugehörige Bild konnte noch nicht entfernt werden.'; }
    catch { error = 'Der Entwurf konnte nicht verworfen werden. Bitte versuche es erneut.'; }
    finally { busy = null; }
  }

  function categoryLabel(id: string) { return categories.find(c => c.id === id)?.label ?? 'Kategorie'; }
</script>

<svelte:head><title>Pocket Finance — Belege einfach im Blick</title></svelte:head>

<div class="app-shell">
  <aside class="sidebar">
    <a class="brand" href="/" aria-label="Pocket Finance Startseite"><span class="brand-mark"><ReceiptText size={23} strokeWidth={1.8} /></span><span>Pocket<span class="brand-light">Finance</span></span></a>
    <div class="sidebar-label">DEIN FINANZALLTAG</div>
    <button class:active={!draft} class="nav-item" onclick={() => { if (draft) askDiscard(); detail = null; }} disabled={Boolean(busy)}><Wallet size={19} /> Übersicht <span class="nav-dot"></span></button>
    <button class:active={Boolean(draft)} class="nav-item" onclick={() => { if (!draft) void start('camera'); }} disabled={Boolean(busy)}><ScanLine size={19} /> Beleg erfassen</button>
    <div class="sidebar-bottom"><ShieldCheck size={19} /><div><strong>Dein Geld. Deine Daten.</strong><span>Lokal auf deinem Gerät.</span></div></div>
  </aside>

  <div class="main-shell">
    <header class="topbar">
      <a class="mobile-brand" href="/" aria-label="Pocket Finance Startseite"><span class="brand-mark"><ReceiptText size={21} /></span><strong>Pocket Finance</strong></a>
      <div class="breadcrumb">Persönlich <ChevronRight size={14} /><span>{draft ? 'Beleg prüfen' : 'Übersicht'}</span></div>
      <span class="offline-status"><span class="status-dot"></span>{native ? 'Auf diesem Gerät' : 'Browser-Vorschau'}</span>
    </header>

    <main>
      {#if !native && initialized}
        <div class="demo-notice"><span class="demo-label">DEMO</span><span>Nur synthetische Belege. {connectedDemo ? 'Die lokale Rust-Testbrücke verarbeitet und speichert diese Testdaten.' : 'Keine echte Kamera oder OCR. Änderungen gelten nur bis zum Neuladen.'}</span></div>
      {/if}

      {#if notice}<div class="inline-note" role="status"><AlertCircle size={19} /><span>{notice}</span></div>{/if}
      {#if error}
        <div class="error-banner" role="alert"><AlertCircle size={20} /><span>{error}</span>{#if !draft}<button onclick={load}>Erneut laden</button>{/if}</div>
      {/if}

      {#if busy === 'capture' || busy === 'ocr'}
        <section class="processing-panel" aria-live="polite" aria-busy="true">
          <div class="processing-icon"><ScanLine size={34} /><LoaderCircle class="spinner progress-spinner" size={18} /></div>
          <p class="eyebrow">BELEG ERFASSEN</p>
          <h1>{busy === 'capture' ? native ? 'Bild auswählen …' : 'Testbeleg öffnen …' : 'Dein Beleg wird gelesen.'}</h1>
          <p>{busy === 'capture' ? 'Nach der Auswahl kannst du alle Angaben prüfen.' : 'Händler, Datum und Gesamtbetrag werden für dich vorbereitet.'}</p>
          <div class="privacy-line"><ShieldCheck size={16} />{native ? 'Die Verarbeitung bleibt auf deinem Gerät.' : 'Synthetischer Testablauf in der Browser-Vorschau.'}</div>
        </section>
      {:else if draft}
        <section class="review-section">
          <div class="page-heading review-heading">
            <div><button class="back-link" onclick={askDiscard} disabled={Boolean(busy)}><ArrowLeft size={16} /> Zur Übersicht</button><h1 id="review-heading" tabindex="-1">Einmal prüfen.<br class="mobile-break" /> Dann ablegen.</h1><p>Dein Beleg ist vorbereitet. Du entscheidest, was gespeichert wird.</p></div>
            <Badge class="draft-badge"><span class="small-dot"></span> Entwurf</Badge>
          </div>
          <div class="review-grid">
            <div class="receipt-column">
              <div class="receipt-preview">
                <div class="preview-toolbar"><span><ReceiptText size={16} /> Originalbeleg</span>{#if photo}<button aria-label="Beleg vergrößern" onclick={() => photoDialog.showModal()}><Expand size={17} /></button>{/if}</div>
                {#if photo}<img src={photo} alt="Aufgenommener Beleg zur Prüfung der erkannten Angaben" />{:else}<div class="no-preview"><ReceiptText size={40} /><p>Keine Bildvorschau verfügbar.</p></div>{/if}
              </div>
              <div class="image-note"><ShieldCheck size={15} /><span>{native ? 'Das Original bleibt lokal gespeichert.' : 'Dieser Beleg ist ein synthetisches Beispiel.'}</span></div>
              {#if draft.ocrText}<details class="ocr-details"><summary>Erkannten Text anzeigen <ChevronRight size={15} /></summary><pre>{draft.ocrText}</pre></details>{/if}
            </div>

            <Card.Root class="review-card">
              <Card.Header class="review-card-header"><div class="section-kicker"><span>01</span> BELEGDATEN</div><Card.Title class="form-title">Die wichtigsten Angaben</Card.Title><Card.Description>Bitte mit deinem Originalbeleg vergleichen.</Card.Description></Card.Header>
              <Card.Content class="review-card-content">
                {#if !draft.ocrText}<div class="inline-note" role="status"><AlertCircle size={19} /><span>Der Text konnte nicht erkannt werden. Du kannst alle Angaben selbst eintragen.</span></div>{/if}
                {#if currency !== 'EUR'}<div class="inline-note" role="alert"><AlertCircle size={19} /><span>Erkannte Währung: {draft.currency}. Nur EUR-Belege können gespeichert werden. Korrigiere die Währung nur, wenn die Erkennung falsch war.</span></div>{:else if currencyUnclear}<div class="inline-note" role="status"><AlertCircle size={19} /><span>Die Währung war nicht eindeutig lesbar. Bitte bestätige am Originalbeleg, dass der Betrag in EUR angegeben ist.</span></div>{/if}
                <form onsubmit={(event) => { event.preventDefault(); void save(); }}>
                  <fieldset disabled={Boolean(busy)} class="edit-fields">
                    <div class="field"><Label for="merchant">Händler</Label><Input id="merchant" name="merchant" class="review-input" bind:value={merchant} oninput={changed} autocomplete="organization" placeholder="Name auf dem Beleg" maxlength={160} required /></div>
                    <div class="field-row"><div class="field"><Label for="receipt-date">Belegdatum</Label><Input id="receipt-date" name="date" class="review-input" type="date" bind:value={receiptDate} oninput={changed} required /></div><div class="field"><Label for="currency">Währung</Label><div class="select-wrap"><select id="currency" bind:value={currency} onchange={changed}><option value="EUR">EUR · Euro</option>{#if draft.currency !== 'EUR'}<option value={draft.currency}>{draft.currency} · erkannt</option>{/if}</select><ChevronRight size={16} /></div><p class="currency-help">Nur EUR. Keine automatische Umrechnung.</p></div></div>
                    <div class="amount-area"><div class="field"><Label for="amount">Gesamtbetrag</Label><div class="amount-input-wrap"><Input id="amount" name="amount" class="review-input amount-input" bind:value={amount} oninput={changed} inputmode="decimal" autocomplete="off" placeholder="0,00" aria-describedby="amount-help" required /><span>€</span></div><p id="amount-help">Der zu zahlende Betrag – ohne gegebenes Bargeld und Rückgeld.</p></div>
                      {#if ambiguous}<fieldset class="candidate-box"><legend><AlertCircle size={16} /> Mehrere mögliche Gesamtbeträge</legend><p>Bitte wähle den richtigen Betrag oder trage ihn oben ein.</p><div class="candidate-options">{#each draft.amountCandidates as candidate}<button type="button" class:selected={amountCents === candidate.amountCents} onclick={() => choose(candidate.amountCents)} aria-pressed={amountCents === candidate.amountCents}><span>{formatMoney(candidate.amountCents)}</span>{#if amountCents === candidate.amountCents}<Check size={16} />{/if}<small>{candidate.label}</small></button>{/each}</div></fieldset>{/if}
                    </div>
                    <div class="field category-field"><Label for="category">Kategorie</Label><div class="select-wrap"><select id="category" name="category" bind:value={categoryId} onchange={changed} required><option value="" disabled>Kategorie auswählen</option>{#each categories as category}<option value={category.id}>{category.label}</option>{/each}</select><ChevronRight size={16} /></div><p class="category-helper">{#if suggesting}<LoaderCircle class="spinner" size={13} /> Optionaler Vorschlag wird gesucht …{:else if suggestion?.status === 'suggested'}<Sparkles size={13} /> Vorschlag erhalten. Du kannst ihn jederzeit ändern.{:else}Wähle die Kategorie, die für dich passt.{/if}</p></div>
                    <label class="confirmation"><input type="checkbox" bind:checked={confirmed} /><span>Ich habe Händler, Datum, Währung, Betrag und Kategorie geprüft und möchte diese Ausgabe speichern.</span></label>
                    <Button type="submit" class="save-button" disabled={!valid || !confirmed || Boolean(busy)}>{#if busy === 'saving'}<LoaderCircle class="spinner" size={18} /> Wird gespeichert …{:else}<Check size={18} /> Ausgabe speichern <span>{amountCents ? formatMoney(amountCents) : '—'}</span>{/if}</Button>
                  </fieldset>
                </form>
                <button class="discard-button" onclick={askDiscard} disabled={Boolean(busy)}>Entwurf verwerfen</button>
              </Card.Content>
            </Card.Root>
          </div>
        </section>
      {:else}
        <section class="overview-section">
          <div class="page-heading"><div><p class="eyebrow">WENIGER PAPIER. MEHR ÜBERBLICK.</p><h1>Alles an einem Ort.</h1><p>Beleg aufnehmen, Angaben prüfen, Ausgabe ablegen.</p></div><Button class="header-scan" onclick={() => start('camera')} disabled={!initialized || Boolean(busy)}><Plus size={17} />{native ? 'Neuer Beleg' : 'Beispielbeleg prüfen'}</Button></div>
          {#if saved}<div class="success-banner" role="status" id="saved-message" tabindex="-1"><span class="success-icon"><CheckCheck size={23} /></span><div><strong>{native || connectedDemo ? 'Ausgabe gespeichert' : 'In der Demo gespeichert'}</strong><p>{saved.merchant} · {formatMoney(saved.totalCents)} · {formatDate(saved.date)}</p></div><button aria-label="Erfolgsmeldung schließen" onclick={() => saved = null}><X size={18} /></button></div>{/if}
          <div class="overview-grid">
            <Card.Root class="balance-card"><Card.Content class="balance-content"><div class="balance-top"><span>Summe der angezeigten Belege</span><span class="balance-icon"><Wallet size={18} /></span></div><div class="balance-amount">{Number.isSafeInteger(totalSpent) ? formatMoney(totalSpent) : '—'}</div><div class="balance-bottom"><span>{transactions.length} {transactions.length === 1 ? 'Beleg' : 'Belege'} gespeichert</span><span>EUR</span></div></Card.Content></Card.Root>
            <Card.Root class="scan-card"><Card.Content class="scan-content"><span class="scan-icon"><ScanLine size={28} strokeWidth={1.6} /></span><div><h2>Aus Papier wird Übersicht.</h2><p>Ein Foto genügt für den ersten Schritt.<br />Alles Weitere prüfst du selbst.</p></div><ArrowUpRight class="scan-arrow" size={24} /></Card.Content></Card.Root>
          </div>
          <div class="list-heading"><div><h2>Deine Belege</h2><span>{transactions.length ? 'Bis zu 200 zuletzt gespeicherte Belege' : 'Hier beginnt deine persönliche Übersicht'}</span></div><Badge variant="outline" class="count-badge">{transactions.length}</Badge></div>
          {#if !initialized}<div class="loading-list" role="status"><LoaderCircle class="spinner" size={24} /> Übersicht wird geladen …</div>{:else if transactions.length === 0}
            <Card.Root class="empty-card"><Card.Content class="empty-content"><div class="receipt-illustration" aria-hidden="true"><div class="receipt-paper"><span class="paper-logo"><Leaf size={20} /></span><span class="paper-line long"></span><span class="paper-line"></span><span class="paper-divider"></span><span class="paper-line long"></span><span class="paper-line short"></span><span class="paper-total"><span></span><Check size={15} /></span></div><div class="receipt-check"><Check size={21} /></div><span class="illustration-spark spark-one">+</span><span class="illustration-spark spark-two">+</span></div><h2>Dein erster Beleg wartet.</h2><p>Halte einen Einkauf fest.<br />Die Angaben bleiben bis zu deiner Bestätigung ein Entwurf.</p><div class="capture-actions"><Button onclick={() => start('camera')} class="capture-primary"><Camera size={18} />{native ? 'Foto aufnehmen' : 'Beispielbeleg prüfen'}</Button><Button variant="outline" onclick={() => start('import')}><ImagePlus size={18} />{native ? 'Bild auswählen' : 'Beispiel importieren'}</Button></div><span class="empty-privacy"><WifiOff size={14} />{native ? 'Kein Konto. Keine Cloud. Deine Kontrolle.' : 'Browser-Demo mit synthetischen Testdaten.'}</span></Card.Content></Card.Root>
          {:else}<div class="transaction-list">{#each transactions as transaction}<button class="transaction-row" onclick={() => detail = detail?.id === transaction.id ? null : transaction} aria-expanded={detail?.id === transaction.id}><span class="transaction-icon"><ReceiptText size={22} /></span><span class="transaction-description"><strong>{transaction.merchant}</strong><span>{categoryLabel(transaction.categoryId)} <span class="dot-separator">·</span> {formatDate(transaction.date)}</span></span><span class="transaction-amount">−{formatMoney(transaction.totalCents)}<span><CheckCircle2 size={12} /> Gespeichert</span></span><ChevronRight size={17} /></button>{#if detail?.id === transaction.id}<div class="transaction-detail"><dl><div><dt>Belegdatum</dt><dd>{formatDate(transaction.date)}</dd></div><div><dt>Kategorie</dt><dd>{categoryLabel(transaction.categoryId)}</dd></div><div><dt>Betrag</dt><dd>{transaction.totalCents} Cent · {transaction.currency}</dd></div></dl><p>Dieser Datensatz wurde nach deiner Bestätigung gespeichert.</p></div>{/if}{/each}</div><div class="saved-capture-actions"><Button onclick={() => start('camera')}><Camera size={18} />{native ? 'Weiteren Beleg aufnehmen' : 'Beispielbeleg prüfen'}</Button><Button variant="outline" onclick={() => start('import')}><ImagePlus size={18} />{native ? 'Bild auswählen' : 'Beispiel importieren'}</Button></div>{/if}
          {#if !native}<details class="demo-controls"><summary>Testszenario auswählen <ChevronRight size={15} /></summary><Label for="demo-scenario">Synthetischer Beleg für den nächsten Durchlauf</Label><select id="demo-scenario" bind:value={scenario}><option value="standard">12,49 € Gesamt · 20,00 € bar · 7,51 € Rückgeld</option><option value="ambiguous">Zwei mögliche Gesamtbeträge</option><option value="ocr_failure">Texterkennung nicht verfügbar</option></select><p>Die Kategorie kann ohne Modell manuell gewählt werden.</p></details>{/if}
          <footer class="page-footer"><ShieldCheck size={15} /><span>Du bestätigst. Erst dann wird gespeichert.</span><span class="footer-wordmark">POCKET FINANCE</span></footer>
        </section>
      {/if}
    </main>
  </div>
</div>

<dialog bind:this={photoDialog} class="photo-dialog" onclick={(event) => { if (event.target === photoDialog) photoDialog.close(); }} onkeydown={(event) => { if (event.key === 'Escape') photoDialog.close(); }}>
  <div class="photo-dialog-header"><span>Originalbeleg</span><Button variant="ghost" size="icon" onclick={() => photoDialog.close()} aria-label="Vergrößerten Beleg schließen"><X size={22} /></Button></div>
  {#if photo}<img src={photo} alt="Vergrößerter Originalbeleg" />{/if}
</dialog>

<dialog bind:this={discardDialog} class="discard-dialog" aria-labelledby="discard-title">
  <h2 id="discard-title">Entwurf verwerfen?</h2>
  <p>Dieser Beleg wurde noch nicht gespeichert. Deine Eingaben werden verworfen.</p>
  <div><Button variant="outline" onclick={() => discardDialog.close()}>Weiter prüfen</Button><Button variant="destructive" onclick={discard}>Entwurf verwerfen</Button></div>
</dialog>
