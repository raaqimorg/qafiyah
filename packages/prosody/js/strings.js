export const charAt = (text, index) => {
  if (!Number.isInteger(index) || index < 0 || index >= text.length) {
    throw new RangeError(`index ${index} out of range for length ${text.length}`);
  }
  return text[index];
};

export const slice = (text, start, end) => {
  if (start < 0 || end > text.length || start > end) {
    throw new RangeError(`range ${start}..${end} out of range for length ${text.length}`);
  }
  return text.substring(start, end);
};
