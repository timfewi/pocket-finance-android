export interface Category { id: string; label: string }
export interface ReceiptImage { imageRef: string; previewRef: string | null }
export interface AmountCandidate { id: string; label: string; amountCents: number; lineIndex: number }
export interface ReceiptDraft {
  id: string;
  imageRef: string;
  previewRef: string | null;
  ocrText: string;
  merchant: string | null;
  date: string | null;
  totalCents: number | null;
  currency: string;
  categoryId: string | null;
  amountCandidates: AmountCandidate[];
  warnings: string[];
  status: 'draft' | 'saved';
}
export interface SaveReceiptRequest {
  draftId: string;
  merchant: string;
  date: string;
  totalCents: number;
  currency: string;
  categoryId: string;
  confirmed: boolean;
}
export interface Transaction {
  id: string;
  draftId: string;
  merchant: string;
  date: string;
  totalCents: number;
  currency: string;
  categoryId: string;
  imageRef: string;
  createdAt: string;
}
export interface CategorySuggestion {
  categoryId: string | null;
  status: 'suggested' | 'unavailable' | 'failed';
}
export type DemoScenario = 'standard' | 'ambiguous' | 'ocr_failure';

export interface DiscardResult { discarded: boolean; imageCleanupPending: boolean }
