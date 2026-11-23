import { describe, expect, it } from 'vitest';
import { readableError, safeHttps } from './safety';

describe('displayed external sources', () => {
  it('accepts https and rejects executable or credential-bearing links', () => {
    expect(safeHttps('https://example.com/a?q=1')).toBe(true);
    for (const value of ['javascript:alert(1)', 'data:text/html,test', 'file:///C:/secret', 'http://example.com', 'https://user:pass@example.com', 'https://example.com:1234']) expect(safeHttps(value)).toBe(false);
  });
  it('keeps backend error messages readable', () => {
    expect(readableError('Model eksik')).toBe('Model eksik');
    expect(readableError(new Error('İşlem durduruldu'))).toBe('İşlem durduruldu');
  });
});
