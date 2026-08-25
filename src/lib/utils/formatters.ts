export function formatBytes(bytes: number, decimals: number = 2): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

export function parseDateTimestamp(dateValue?: string | number | null): number {
  if (!dateValue) return 0;
  const num = typeof dateValue === 'number' ? dateValue : Number(dateValue);
  if (!isNaN(num) && num > 0) {
    return num < 10_000_000_000 ? num * 1000 : num;
  }
  const t = new Date(String(dateValue)).getTime();
  return isNaN(t) ? 0 : t;
}

export function formatDate(dateValue?: string | number | null): string {
  if (!dateValue) return '—';
  try {
    const num = typeof dateValue === 'number' ? dateValue : Number(dateValue);
    const date = !isNaN(num) && num > 0
      ? new Date(num < 10_000_000_000 ? num * 1000 : num)
      : new Date(String(dateValue));
    if (isNaN(date.getTime())) return String(dateValue);
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  } catch {
    return String(dateValue);
  }
}

export function cleanPostTitle(title?: string | null): string {
  if (!title) return '';
  let text = String(title);

  // Decode HTML entities
  text = text
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&apos;/g, "'")
    .replace(/&nbsp;/g, ' ');

  // Strip HTML tags
  text = text.replace(/<[^>]*>/g, ' ');

  // Strip markdown formatting while keeping text content
  text = text
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/(\*{1,2}|_{1,2})(.*?)\1/g, '$2')
    .replace(/~~(.*?)~~/g, '$1')
    .replace(/`([^`]+)`/g, '$1');

  // Normalize whitespace
  return text.replace(/\s+/g, ' ').trim();
}

export function parseTags(tagsValue?: any): string[] {
  if (!tagsValue) return [];
  if (Array.isArray(tagsValue)) {
    return tagsValue.map((t) => String(t).trim()).filter(Boolean);
  }
  if (typeof tagsValue === 'string') {
    const trimmed = tagsValue.trim();
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      return trimmed
        .slice(1, -1)
        .split(',')
        .map((t) => t.trim().replace(/^"(.*)"$/, '$1'))
        .filter(Boolean);
    }
    if (trimmed.includes(',')) {
      return trimmed.split(',').map((t) => t.trim()).filter(Boolean);
    }
    if (trimmed) return [trimmed];
  }
  return [];
}
