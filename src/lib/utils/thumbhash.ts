import { thumbHashToDataURL } from 'thumbhash';

const cache = new Map<string, string>();

export function thumbHashToUrl(base64Hash?: string | null): string | null {
  if (!base64Hash || typeof base64Hash !== 'string') return null;
  const hit = cache.get(base64Hash);
  if (hit) return hit;

  try {
    const binary = Uint8Array.from(atob(base64Hash), (c) => c.charCodeAt(0));
    const dataUrl = thumbHashToDataURL(binary);
    cache.set(base64Hash, dataUrl);
    return dataUrl;
  } catch {
    return null;
  }
}
