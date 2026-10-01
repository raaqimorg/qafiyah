import { describe, expect, it, vi } from 'vitest';

import { createCsvFilterSetter, splitCsvIds, validateText } from './csv-filters';

describe('validateText', () => {
  it('passes text at or under the query length limit', () => {
    expect(validateText('حب')).toBeNull();
    expect(validateText('ا'.repeat(100))).toBeNull();
  });

  it('returns the localized error past the limit', () => {
    expect(validateText('ا'.repeat(101))).toContain('100');
  });
});

describe('splitCsvIds', () => {
  it('splits a comma string and trims each id', () => {
    expect(splitCsvIds('jahili, abbasi ,')).toEqual(['jahili', 'abbasi']);
  });

  it('passes an array through, dropping blank entries', () => {
    expect(splitCsvIds(['a', '', 'b'])).toEqual(['a', 'b']);
    expect(splitCsvIds(['a', ' '])).toEqual(['a']);
  });

  it('returns an empty list for an empty string', () => {
    expect(splitCsvIds('')).toEqual([]);
  });
});

describe('createCsvFilterSetter', () => {
  it('joins an array and calls the setter with the joined value', () => {
    const setter = vi.fn();
    createCsvFilterSetter(setter)(['a', ' b']);
    expect(setter).toHaveBeenCalledWith('a,b');
  });

  it('calls the setter with null for an empty selection', () => {
    const setter = vi.fn();
    createCsvFilterSetter(setter)('');
    expect(setter).toHaveBeenCalledWith(null);
  });
});
