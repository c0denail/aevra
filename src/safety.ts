export function safeHttps(raw: string): boolean {
  try { const url = new URL(raw); return url.protocol === 'https:' && !url.username && !url.password && (!url.port || url.port === '443'); }
  catch { return false; }
}

export function readableError(error: unknown): string {
  return error instanceof Error ? error.message : typeof error === 'string' ? error : 'İşlem tamamlanamadı. Tekrar deneyin.';
}
