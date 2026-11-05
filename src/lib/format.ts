export function formatMoney(cents: number, currency = 'EUR'): string {
  return new Intl.NumberFormat('de-AT', { style: 'currency', currency }).format(cents / 100);
}
export function editMoney(cents: number | null): string {
  return cents === null ? '' : `${Math.trunc(cents / 100)},${String(cents % 100).padStart(2, '0')}`;
}
/** UI convenience only. Rust validates the canonical integer again before saving. */
export function inputCents(value: string): number | null {
  const match = value.trim().match(/^(\d{1,10})(?:[,.](\d{1,2}))?$/);
  if (!match) return null;
  const cents = Number(match[1]) * 100 + Number((match[2] ?? '').padEnd(2, '0'));
  return Number.isSafeInteger(cents) && cents > 0 ? cents : null;
}
export function formatDate(value: string): string {
  const parts = value.split('-');
  return parts.length === 3 ? `${parts[2]}.${parts[1]}.${parts[0]}` : value;
}
