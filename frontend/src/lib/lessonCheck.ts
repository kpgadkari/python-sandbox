export function normalizeStdout(value: string) {
  return value.replace(/\r\n/g, '\n').trimEnd();
}

export function expectedOutputMatches(actual: string, expected: string) {
  return normalizeStdout(actual) === normalizeStdout(expected);
}
