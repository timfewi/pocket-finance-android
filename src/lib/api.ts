import { invoke, isTauri, convertFileSrc } from '@tauri-apps/api/core';
import type { Category, CategorySuggestion, DemoScenario, DiscardResult, ReceiptDraft, ReceiptImage, SaveReceiptRequest, Transaction } from './types';

const localCategories: Category[] = [
  { id: 'groceries', label: 'Lebensmittel' },
  { id: 'dining', label: 'Essen & Trinken' },
  { id: 'transport', label: 'Mobilität' },
  { id: 'household', label: 'Haushalt' },
  { id: 'health', label: 'Gesundheit' },
  { id: 'shopping', label: 'Einkäufe' },
  { id: 'subscriptions', label: 'Abonnements' },
  { id: 'other', label: 'Sonstiges' }
];
const standardText = 'ALPENMARKT\nMusterstraße 12, 1010 Wien\nDEMOBELEG · SYNTHETISCH\n05.11.2026 14:32\nBrot 3,49\nMilch 2,50\nObst 6,50\nGESAMT EUR 12,49\nBAR 20,00\nRÜCKGELD 7,51';
const drafts = new Map<string, ReceiptDraft>();
const demoTransactions: Transaction[] = [];
const savedRequests = new Map<string, string>();
const scenarioForImage = new Map<string, DemoScenario>();

export function nativeRuntime(): boolean { return isTauri(); }
export function browserBridge(): boolean { return !isTauri() && Boolean(import.meta.env.DEV && import.meta.env.VITE_RECEIPT_BRIDGE_URL); }

async function request<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isTauri()) return invoke<T>(command, args);
  // This branch is compiled away from production builds. Native code never sends OCR to HTTP.
  if (import.meta.env.DEV && import.meta.env.VITE_RECEIPT_BRIDGE_URL) {
    const url = new URL(import.meta.env.VITE_RECEIPT_BRIDGE_URL);
    if (!['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname) || url.protocol !== 'http:') {
      throw new Error('Die Testbrücke muss lokal laufen.');
    }
    const response = await fetch(`${url.origin}/invoke`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args })
    });
    if (!response.ok) throw new Error('Die lokale Testbrücke konnte die Anfrage nicht verarbeiten.');
    return response.json() as Promise<T>;
  }
  return demoRequest(command, args) as T;
}

function demoRequest(command: string, args: Record<string, unknown>): unknown {
  if (command === 'list_categories') return localCategories;
  if (command === 'list_transactions') return [...demoTransactions];
  if (command === 'capture_receipt' || command === 'import_receipt') {
    const imageRef = `demo-${crypto.randomUUID()}`;
    scenarioForImage.set(imageRef, (args.scenario as DemoScenario | undefined) ?? 'standard');
    return { imageRef, previewRef: args.scenario === 'ambiguous' ? '/demo-receipt-ambiguous.svg' : '/demo-receipt.svg' };
  }
  if (command === 'draft_from_receipt') {
    const imageRef = String(args.imageRef);
    const scenario = scenarioForImage.get(imageRef) ?? 'standard';
    const ambiguous = scenario === 'ambiguous';
    const failed = scenario === 'ocr_failure';
    const draft: ReceiptDraft = {
      id: crypto.randomUUID(), imageRef, previewRef: ambiguous ? '/demo-receipt-ambiguous.svg' : '/demo-receipt.svg',
      ocrText: failed ? '' : ambiguous ? standardText.replace('GESAMT EUR 12,49', 'GESAMT EUR 12,49\nGESAMTBETRAG EUR 14,49') : standardText,
      merchant: failed ? null : 'Alpenmarkt', date: failed ? null : '2026-11-05',
      totalCents: failed || ambiguous ? null : 1249, currency: 'EUR', categoryId: null,
      amountCandidates: failed ? [] : ambiguous
        ? [{ id: 'total-1', label: 'GESAMT EUR 12,49', amountCents: 1249, lineIndex: 7 }, { id: 'total-2', label: 'GESAMTBETRAG EUR 14,49', amountCents: 1449, lineIndex: 8 }]
        : [{ id: 'total-1', label: 'GESAMT EUR 12,49', amountCents: 1249, lineIndex: 7 }],
      warnings: failed ? ['ocr_unavailable', 'missing_merchant', 'missing_date', 'missing_total'] : ambiguous ? ['ambiguous_total'] : [],
      status: 'draft'
    };
    drafts.set(draft.id, draft);
    return draft;
  }
  if (command === 'suggest_category') return { categoryId: null, status: 'unavailable' };
  if (command === 'discard_receipt') { drafts.delete(String(args.draftId)); return { discarded: true, imageCleanupPending: false }; }
  if (command === 'save_receipt') {
    const data = args.request as SaveReceiptRequest;
    const fingerprint = JSON.stringify(data);
    const saved = demoTransactions.find(row => row.draftId === data.draftId);
    if (saved) {
      if (savedRequests.get(data.draftId) !== fingerprint) throw new Error('Dieser Beleg wurde bereits mit anderen Angaben gespeichert.');
      return saved;
    }
    if (!drafts.has(data.draftId) || !data.confirmed || !data.merchant.trim() || !data.date || data.currency !== 'EUR' || !Number.isSafeInteger(data.totalCents) || data.totalCents <= 0 || !localCategories.some(c => c.id === data.categoryId)) {
      throw new Error('Bitte prüfe die Angaben vor dem Speichern.');
    }
    const row: Transaction = { ...data, id: crypto.randomUUID(), imageRef: drafts.get(data.draftId)!.imageRef, createdAt: new Date().toISOString() };
    demoTransactions.unshift(row);
    savedRequests.set(data.draftId, fingerprint);
    return row;
  }
  throw new Error('Diese Funktion ist in der Browser-Demo nicht verfügbar.');
}

export const listCategories = () => request<Category[]>('list_categories');
export const listTransactions = () => request<Transaction[]>('list_transactions');
export const captureReceipt = (scenario: DemoScenario) => request<ReceiptImage>('capture_receipt', isTauri() ? {} : { scenario });
export const importReceipt = (scenario: DemoScenario) => request<ReceiptImage>('import_receipt', isTauri() ? {} : { scenario });
export const draftFromReceipt = (imageRef: string) => request<ReceiptDraft>('draft_from_receipt', { imageRef });
export const suggestCategory = (draftId: string) => request<CategorySuggestion>('suggest_category', { draftId });
export const saveReceipt = (data: SaveReceiptRequest) => request<Transaction>('save_receipt', { request: data });
export const discardReceipt = (draftId: string) => request<DiscardResult>('discard_receipt', { draftId });
export function previewUrl(reference: string | null): string | null {
  if (!reference) return null;
  if (isTauri()) return /^[a-zA-Z0-9-]+$/.test(reference) ? convertFileSrc(reference, 'receipt') : null;
  if (reference.startsWith('/demo-')) return reference;
  if (import.meta.env.DEV && import.meta.env.VITE_RECEIPT_BRIDGE_URL) {
    try {
      const url = new URL(reference);
      const bridge = new URL(import.meta.env.VITE_RECEIPT_BRIDGE_URL);
      if (url.origin === bridge.origin && url.pathname.startsWith('/fixtures/')) return url.toString();
    } catch { /* Untrusted preview references are not rendered. */ }
  }
  return null;
}
