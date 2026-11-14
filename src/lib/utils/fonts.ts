const GENERIC_FAMILIES = new Set([
  'serif',
  'sans-serif',
  'monospace',
  'cursive',
  'fantasy',
  'system-ui',
  'ui-serif',
  'ui-sans-serif',
  'ui-monospace',
  'ui-rounded',
  'emoji',
  'math',
  'fangsong',
  '-apple-system',
  'blinkmacsystemfont',
  'inherit',
  'initial',
  'unset',
  'revert',
  'revert-layer'
]);

function familyName(raw: string): string {
  return raw.trim().replace(/^['"]+|['"]+$/g, '').trim();
}

export function isFontFamilyAvailable(family: string): boolean {
  const name = familyName(family);
  if (!name) return true;
  if (GENERIC_FAMILIES.has(name.toLowerCase())) return true;
  if (typeof document === 'undefined') return true;

  const context = document.createElement('canvas').getContext('2d');
  if (!context) return true;

  const sample = 'mmmmmmmmmmlli';
  const measure = (font: string) => {
    context.font = font;
    return context.measureText(sample).width;
  };
  const quoted = `"${name.replace(/["\\]/g, '\\$&')}"`;
  const mono = measure('72px monospace');
  const sans = measure('72px sans-serif');
  return measure(`72px ${quoted}, monospace`) !== mono ||
    measure(`72px ${quoted}, sans-serif`) !== sans;
}

export function missingFontFamily(input: string): string | null {
  const families = input.split(',').map(familyName).filter(Boolean);
  if (families.length === 0) return null;
  return families.some(isFontFamilyAvailable) ? null : families[0];
}
