import type { Post, Attachment } from '$lib/types/content';
import { providerState } from '$lib/state/providerState.svelte';
import { getProviderDriver, deriveSubdomainOrigin } from '$lib/providers/drivers';
import { thumbHashToUrl } from './thumbhash';

export { deriveSubdomainOrigin };

export function formatProviderName(name?: string): string {
  if (!name) return '';
  return name.replace(/\s*\([^)]*\)/g, '').trim();
}

function resolveDriver(service?: string, explicitProviderId?: string) {
  if (explicitProviderId) {
    const config = providerState.getProviderById(explicitProviderId);
    if (config) {
      return { config, driver: getProviderDriver(config.id) };
    }
  }
  return providerState.getDriverForService(service);
}

export function cleanMediaPath(rawPath: string): string {
  return rawPath.replace(/^\/*data\//, '').replace(/^\/+/, '');
}

const MIME_MAP: Record<string, string> = {
  'audio/mpeg': 'mp3',
  'audio/mp3': 'mp3',
  'audio/mp4': 'm4a',
  'audio/x-m4a': 'm4a',
  'audio/aac': 'aac',
  'audio/flac': 'flac',
  'audio/x-flac': 'flac',
  'audio/ogg': 'ogg',
  'audio/opus': 'opus',
  'audio/wav': 'wav',
  'audio/x-wav': 'wav',
  'audio/webm': 'weba',
  'video/mp4': 'mp4',
  'video/webm': 'webm',
  'video/quicktime': 'mov',
  'video/x-matroska': 'mkv',
  'video/x-msvideo': 'avi',
  'video/x-flv': 'flv',
  'video/x-ms-wmv': 'wmv',
  'video/3gpp': '3gp',
  'video/ogg': 'ogv',
  'image/jpeg': 'jpg',
  'image/jpg': 'jpg',
  'image/png': 'png',
  'image/webp': 'webp',
  'image/gif': 'gif',
  'image/avif': 'avif',
  'image/bmp': 'bmp',
  'image/svg+xml': 'svg',
  'image/tiff': 'tiff',
  'image/heic': 'heic',
  'image/heif': 'heif',
  'image/x-icon': 'ico',
  'image/vnd.adobe.photoshop': 'psd',
  'image/x-photoshop': 'psd',
  'application/x-photoshop': 'psd',
  'application/zip': 'zip',
  'application/x-zip-compressed': 'zip',
  'application/x-rar-compressed': 'rar',
  'application/x-rar': 'rar',
  'application/vnd.rar': 'rar',
  'application/x-7z-compressed': '7z',
  'application/x-tar': 'tar',
  'application/gzip': 'gz',
  'application/x-bzip2': 'bz2',
  'application/x-xz': 'xz',
  'application/pdf': 'pdf',
  'application/epub+zip': 'epub',
  'application/x-blender': 'blend',
  'text/plain': 'txt',
  'text/html': 'html',
  'text/markdown': 'md',
  'application/json': 'json',
  'application/xml': 'xml'
};

export function inferAttachmentExtension(file?: { name?: string; path?: string; extra?: any; [key: string]: any } | null, fallback = 'jpg'): string {
  if (!file) return fallback;

  const nameExt = file.name?.split('.').pop()?.split(/[?#]/)[0].trim().toLowerCase();
  if (nameExt && nameExt.length <= 8 && !nameExt.includes('/')) return nameExt;

  const pathExt = file.path?.split('.').pop()?.split(/[?#]/)[0].trim().toLowerCase();
  if (pathExt && pathExt.length <= 8 && !pathExt.includes('/')) return pathExt;

  const variants = (file as any).variants || (file.extra as any)?.variants;
  if (Array.isArray(variants) && variants.length > 0) {
    for (const v of variants) {
      const vName = typeof v === 'string' ? v : v?.name;
      const vExt = vName?.split('.').pop()?.split(/[?#]/)[0].trim().toLowerCase();
      if (vExt && vExt.length <= 8 && !vExt.includes('/')) return vExt;
    }
  }

  const mime = String(
    (file as any).mime_type ||
    (file as any).mimeType ||
    (file.extra as any)?.mime_type ||
    (file.extra as any)?.mimeType ||
    ''
  ).trim().toLowerCase();
  if (mime) {
    if (MIME_MAP[mime]) return MIME_MAP[mime];
    const sub = mime.split('/')[1]?.split(';')[0]?.replace(/^x-/, '').trim();
    if (sub && sub.length <= 8 && sub !== 'octet-stream') return sub;
  }

  const kind = String(
    (file as any).kind ||
    (file as any).type ||
    (file as any).media_type ||
    (file.extra as any)?.kind ||
    (file.extra as any)?.type ||
    (file.extra as any)?.media_type ||
    ''
  ).trim().toLowerCase();
  if (kind === 'video') return 'mp4';
  if (kind === 'audio') return 'mp3';
  if (kind === 'archive') return 'zip';
  if (kind === 'document') return 'pdf';
  if (kind === 'image' || kind === 'gif' || kind === 'photo') return 'jpg';

  return fallback;
}

export function creatorAvatarUrl(service: string, creatorId: string, thumbhash?: string | null): string {
  if (thumbhash) {
    const dataUrl = thumbHashToUrl(thumbhash);
    if (dataUrl) return dataUrl;
  }
  const { config, driver } = resolveDriver(service);
  return driver.resolveAvatarUrl(config, service, creatorId);
}

export function creatorBannerUrl(service: string, creatorId: string, thumbhash?: string | null): string {
  if (thumbhash) {
    const dataUrl = thumbHashToUrl(thumbhash);
    if (dataUrl) return dataUrl;
  }
  const { config, driver } = resolveDriver(service);
  return driver.resolveBannerUrl(config, service, creatorId);
}

export function creatorPageUrl(service: string, creatorId: string): string {
  const { config, driver } = resolveDriver(service);
  return driver.resolveCreatorPageUrl(config, service, creatorId);
}

export function postPageUrl(service: string, creatorId: string, postId: string): string {
  const { config, driver } = resolveDriver(service);
  return driver.resolvePostPageUrl(config, service, creatorId, postId);
}

export function postMediaUrl(post: Post): string | null {
  const media = post.file?.path ? post.file : post.attachments?.find((item) => item.path);
  if (!media?.path) return null;
  return attachmentMediaUrl(media, post.service);
}

export function resolveServerOrigin(server: string, service?: string): string {
  const srv = server.trim().replace(/\/+$/, '');
  if (/^https?:\/\//i.test(srv)) return srv;
  if (srv.includes('.')) return `https://${srv}`;

  const { config } = resolveDriver(service);
  const providerOrigin = config.file_url ? config.file_url : deriveSubdomainOrigin(config.api_url, 'file');
  try {
    const url = new URL(providerOrigin);
    const host = url.hostname;
    const parts = host.split('.');
    const baseHost = parts.length > 2 ? parts.slice(1).join('.') : host;
    return `${url.protocol}//${srv}.${baseHost}`;
  } catch {
    return providerOrigin;
  }
}

export function attachmentMediaUrl(file: Attachment, service: string): string {
  if (!file?.path) return '';
  if (file.path.startsWith('http://') || file.path.startsWith('https://') || file.path.startsWith('/cloud_stream/')) {
    return file.path;
  }

  const isPreviewOnly = (file as any).preview_only === true || (file.extra as any)?.preview_only === true;
  if (isPreviewOnly) {
    return attachmentThumbnailUrl(file, service);
  }

  const explicitProv = (file as any)?.provider_id || (file.extra as any)?.provider_id;
  const { config, driver } = resolveDriver(service, explicitProv);
  const ext = inferAttachmentExtension(file);
  return driver.resolveMediaUrl(config, file.path, file.server, ext);
}

export function attachmentThumbnailUrl(file: Attachment, service: string): string {
  if (!file?.path) return '';
  if (isAttachmentVideo(file, file.path)) {
    const thumb = (file as any)?.thumbnail || (file as any)?.preview || (file.extra as any)?.thumbnail || (file.extra as any)?.preview;
    if (thumb && typeof thumb === 'string') return thumb;
    const explicitProv = (file as any)?.provider_id || (file.extra as any)?.provider_id;
    const { config, driver } = resolveDriver(service, explicitProv);
    if (config.id === 'onlyhaven') {
      return driver.resolveThumbnailUrl(config, file.path);
    }
    return '';
  }
  if (file.path.startsWith('/cloud_stream/')) {
    return '';
  }
  if (file.path.startsWith('http://') || file.path.startsWith('https://')) {
    return file.path;
  }

  const explicitProv = (file as any)?.provider_id || (file.extra as any)?.provider_id;
  const { config, driver } = resolveDriver(service, explicitProv);
  return driver.resolveThumbnailUrl(config, file.path);
}

export function postThumbnailUrl(post: Post): string | null {
  const media = post.file?.path ? post.file : post.attachments?.find((item) => item.path);
  if (media?.path) {
    const explicitProv = (post as any)?.provider_id || (post.extra as any)?.provider_id || (media as any)?.provider_id || (media?.extra as any)?.provider_id;
    const { config, driver } = resolveDriver(post.service, explicitProv);
    if (isAttachmentVideo(media, media.path) && config.id !== 'onlyhaven') {
      const thumb = (media as any)?.thumbnail || (media as any)?.preview || (media.extra as any)?.thumbnail || (media.extra as any)?.preview;
      if (thumb && typeof thumb === 'string') return thumb;
      return null;
    }
    return driver.resolveThumbnailUrl(config, media.path);
  }

  const thumbhash = (media as any)?.preview_thumbhash ||
    (media?.extra as any)?.preview_thumbhash ||
    (post.file as any)?.preview_thumbhash ||
    (post.file?.extra as any)?.preview_thumbhash ||
    (post.attachments?.[0] as any)?.preview_thumbhash ||
    (post.attachments?.[0]?.extra as any)?.preview_thumbhash ||
    (post.extra as any)?.preview_thumbhash;

  if (thumbhash) {
    const dataUrl = thumbHashToUrl(thumbhash);
    if (dataUrl) return dataUrl;
  }

  return null;
}

export function postPlaceholderUrl(post: Post): string | null {
  const media = post.file?.path ? post.file : post.attachments?.[0];
  const thumbhash = (media as any)?.preview_thumbhash ||
    (media?.extra as any)?.preview_thumbhash ||
    (post.file as any)?.preview_thumbhash ||
    (post.file?.extra as any)?.preview_thumbhash ||
    (post.attachments?.[0] as any)?.preview_thumbhash ||
    (post.attachments?.[0]?.extra as any)?.preview_thumbhash ||
    (post.extra as any)?.preview_thumbhash;

  if (thumbhash) {
    return thumbHashToUrl(thumbhash);
  }
  return null;
}

export function fancardMediaUrl(card: { hash?: string; ext?: string; mime?: string }, service: string): string {
  if (!card.hash || card.hash.length < 4) return '';
  const ext = (card.ext || '').replace(/^\.+/, '') || (card.mime?.includes('png') ? 'png' : card.mime?.includes('webp') ? 'webp' : card.mime?.includes('gif') ? 'gif' : 'jpg');
  const { config, driver } = resolveDriver(service);
  return driver.resolveFancardMediaUrl(config, service, { hash: card.hash, ext });
}

export function fancardThumbnailUrl(card: { hash?: string; ext?: string; mime?: string; ihash?: string }, service: string): string {
  if (card.hash && card.hash.length >= 4) {
    const ext = (card.ext || '').replace(/^\.+/, '') || (card.mime?.includes('png') ? 'png' : card.mime?.includes('webp') ? 'webp' : card.mime?.includes('gif') ? 'gif' : 'jpg');
    const { config, driver } = resolveDriver(service);
    return driver.resolveFancardThumbnailUrl(config, service, { hash: card.hash, ext });
  }
  if (card.ihash) {
    const dataUrl = thumbHashToUrl(card.ihash);
    if (dataUrl) return dataUrl;
  }
  return '';
}

export function postAttachmentCount(post: Post): number {
  return post.attachment_count ?? post.attachments?.length ?? 0;
}

export function isVideoUrl(url: string | null): boolean {
  if (!url) return false;
  return /\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)(?:$|\?)/i.test(url);
}

export function isImageUrl(url: string | null): boolean {
  if (!url) return false;
  return /\.(avif|bmp|gif|jpe?g|png|webp)(?:$|\?)/i.test(url);
}

export function isAttachmentVideo(file?: Attachment | null, url?: string | null): boolean {
  if (!file && !url) return false;
  if (url && isVideoUrl(url)) return true;
  const name = (file?.name || '').toLowerCase();
  if (/\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(name)) return true;
  const path = (file?.path || '').toLowerCase();
  if (/\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(path)) return true;

  const variants = (file as any)?.variants || (file?.extra as any)?.variants;
  if (Array.isArray(variants)) {
    for (const v of variants) {
      const vName = typeof v === 'string' ? v : v?.name;
      if (vName && /\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(vName)) return true;
    }
  }

  const kind = String(
    (file as any)?.kind ||
    (file as any)?.type ||
    (file as any)?.media_type ||
    (file?.extra as any)?.kind ||
    (file?.extra as any)?.type ||
    (file?.extra as any)?.media_type ||
    ''
  ).toLowerCase();
  if (kind === 'video') return true;

  const mime = String(
    (file as any)?.mime_type ||
    (file as any)?.mimeType ||
    (file?.extra as any)?.mime_type ||
    (file?.extra as any)?.mimeType ||
    ''
  ).toLowerCase();
  if (mime.includes('video') || mime.includes('mp4')) return true;

  return false;
}

export function isAttachmentImage(file?: Attachment | null, url?: string | null): boolean {
  if (!file && !url) return false;
  if (url && isImageUrl(url)) return true;
  const name = (file?.name || '').toLowerCase();
  if (/\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(name)) return true;
  const path = (file?.path || '').toLowerCase();
  if (/\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(path)) return true;

  const variants = (file as any)?.variants || (file?.extra as any)?.variants;
  if (Array.isArray(variants)) {
    for (const v of variants) {
      const vName = typeof v === 'string' ? v : v?.name;
      if (vName && /\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(vName)) return true;
    }
  }

  const kind = String(
    (file as any)?.kind ||
    (file as any)?.type ||
    (file as any)?.media_type ||
    (file?.extra as any)?.kind ||
    (file?.extra as any)?.type ||
    (file?.extra as any)?.media_type ||
    ''
  ).toLowerCase();
  if (kind === 'image' || kind === 'gif' || kind === 'photo') return true;

  const mime = String(
    (file as any)?.mime_type ||
    (file as any)?.mimeType ||
    (file?.extra as any)?.mime_type ||
    (file?.extra as any)?.mimeType ||
    ''
  ).toLowerCase();
  if (mime.includes('image')) return true;

  if ((file as any)?.preview_thumbhash || (file?.extra as any)?.preview_thumbhash) {
    if (!isAttachmentVideo(file, url) && !isAttachmentAudio(file, url)) return true;
  }

  return false;
}

export function isAttachmentAudio(file?: Attachment | null, url?: string | null): boolean {
  if (!file && !url) return false;
  if (url && isAudioUrl(url)) return true;
  const name = (file?.name || '').toLowerCase();
  if (/\.(mp3|m4a|aac|wav|ogg|opus|flac)(?:$|[?#])/i.test(name)) return true;
  const path = (file?.path || '').toLowerCase();
  if (/\.(mp3|m4a|aac|wav|ogg|opus|flac)(?:$|[?#])/i.test(path)) return true;

  const variants = (file as any)?.variants || (file?.extra as any)?.variants;
  if (Array.isArray(variants)) {
    for (const v of variants) {
      const vName = typeof v === 'string' ? v : v?.name;
      if (vName && /\.(mp3|m4a|aac|wav|ogg|opus|flac)(?:$|[?#])/i.test(vName)) return true;
    }
  }

  const kind = String(
    (file as any)?.kind ||
    (file as any)?.type ||
    (file as any)?.media_type ||
    (file?.extra as any)?.kind ||
    (file?.extra as any)?.type ||
    (file?.extra as any)?.media_type ||
    ''
  ).toLowerCase();
  if (kind === 'audio') return true;

  const mime = String(
    (file as any)?.mime_type ||
    (file as any)?.mimeType ||
    (file?.extra as any)?.mime_type ||
    (file?.extra as any)?.mimeType ||
    ''
  ).toLowerCase();
  if (mime.includes('audio')) return true;

  return false;
}

export function isAudioUrl(url: string | null): boolean {
  if (!url) return false;
  return /\.(flac|m4a|mp3|ogg|opus|wav)(?:$|\?)/i.test(url);
}

export function isCompressedUrl(url: string | null): boolean {
  if (!url) return false;
  return /\.(7z|gz|rar|tar|tar\.gz|tar\.xz|zip|zipx)(?:$|\?)/i.test(url);
}

export function isDocumentUrl(url: string | null): boolean {
  if (!url) return false;
  return /\.(epub|pdf|txt)(?:$|\?)/i.test(url);
}

export function getPostFormats(post: Post): string[] {
  const p = post;
  const allFiles: string[] = [];
  if (p.file?.path) allFiles.push(p.file.path.toLowerCase());
  if (p.file?.name) allFiles.push(p.file.name.toLowerCase());
  if (p.attachments && Array.isArray(p.attachments)) {
    for (const att of p.attachments) {
      if (att?.path) allFiles.push(att.path.toLowerCase());
      if (att?.name) allFiles.push(att.name.toLowerCase());
    }
  }

  const directMedia = extractDirectMediaLinks(p.content || p.substring || '');
  for (const d of directMedia) {
    if (d.url) allFiles.push(d.url.toLowerCase());
    if (d.name) allFiles.push(d.name.toLowerCase());
  }

  const items = [p.file, ...(p.attachments || [])].filter(Boolean) as Attachment[];
  const hasVideoAttachment = items.some((it) => isAttachmentVideo(it, it.path));
  const hasImageAttachment = items.some((it) => isAttachmentImage(it, it.path));
  const hasAudioAttachment = items.some((it) => isAttachmentAudio(it, it.path));

  const embedStr = JSON.stringify(p.embed || {}).toLowerCase();
  const contentStr = (p.content || p.substring || '').toLowerCase();
  const titleStr = (p.title || '').toLowerCase();
  const tagsStr = Array.isArray(p.tags)
    ? p.tags.join(' ').toLowerCase()
    : typeof p.tags === 'string'
      ? p.tags.toLowerCase()
      : '';

  const formats: string[] = [];

  // Video
  const hasVideoFile = allFiles.some((f) => /\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(f));
  const hasVideoEmbed = /youtube|youtu\.be|vimeo|bilibili|streamable|gfycat|sproutvideo|vids\.io|redgifs|mediadelivery\.net|b-cdn\.net|\.(mp4|webm|mkv|mov|m4v)/i.test(embedStr) || /<video|\.(mp4|webm|mkv|mov|m4v)/i.test(contentStr);
  const hasVideoTitle = /\b(video|mp4|webm|movie|animation|anim|clip|mkv|mov|4k|1080p|720p|60fps|short|pv|trailer)\b/i.test(titleStr);
  const hasVideoTag = /\b(video|mp4|webm|movie|animation|anim|clip|mkv|mov|4k|1080p|720p|60fps|short|pv|trailer)\b/i.test(tagsStr);
  const hasDirectVideo = directMedia.some((d) => isVideoUrl(d.url) || isAttachmentVideo(null, d.url));
  if (hasVideoAttachment || hasVideoFile || hasVideoEmbed || hasVideoTitle || hasVideoTag || hasDirectVideo) {
    formats.push('video');
  }

  // Image
  const hasImageFile = allFiles.some((f) => /\.(avif|bmp|gif|jpe?g|png|webp)(?:$|[?#])/i.test(f));
  const hasImageEmbed = /\.(avif|bmp|gif|jpe?g|png|webp)|<img/i.test(embedStr) || /<img/i.test(contentStr);
  const hasImageTag = /\b(photo|photos|image|images|pic|pics|picture|pictures|illustration|art|cg|drawing|wallpaper)\b/i.test(tagsStr);
  const hasDirectImage = directMedia.some((d) => isImageUrl(d.url) || isAttachmentImage(null, d.url));
  const isGenericMediaImage = (p.file?.path || p.file?.name) && !hasVideoAttachment && !hasAudioAttachment && !hasVideoFile;
  if (hasImageAttachment || hasImageFile || hasImageEmbed || hasImageTag || hasDirectImage || isGenericMediaImage) {
    formats.push('image');
  }

  // Audio
  const hasAudioFile = allFiles.some((f) => /\.(mp3|wav|ogg|m4a|flac|aac|opus|wma)(?:$|[?#])/i.test(f));
  const hasAudioEmbed = /soundcloud|bandcamp|spotify|audio|\.(mp3|wav|ogg|m4a|flac)/i.test(embedStr) || /<audio|\.(mp3|wav|ogg|m4a|flac)/i.test(contentStr);
  const hasAudioTitle = /\b(audio|mp3|wav|flac|sound|track|voice|podcast|asmr|song|music|ost)\b/i.test(titleStr);
  const hasAudioTag = /\b(audio|mp3|wav|flac|sound|track|voice|podcast|asmr|song|music|ost)\b/i.test(tagsStr);
  const hasDirectAudio = directMedia.some((d) => isAudioUrl(d.url) || isAttachmentAudio(null, d.url));
  if (hasAudioAttachment || hasAudioFile || hasAudioEmbed || hasAudioTitle || hasAudioTag || hasDirectAudio) {
    formats.push('audio');
  }

  // Text
  const hasTextContent = Boolean(p.content && p.content.trim().length > 20);
  const isTextOnlyPost = (p.attachment_count ?? 0) === 0 && !p.file?.path && !p.file?.name && directMedia.length === 0;
  if (hasTextContent || isTextOnlyPost) {
    formats.push('text');
  }

  // Archive / Files
  const hasArchiveFile = allFiles.some((f) => /\.(zip|rar|7z|tar|gz|pdf|txt|epub|html|cbz|cbr|psd|clip|blend|fbx|obj|stl)(?:$|[?#])/i.test(f));
  const hasArchiveLink = /mega\.nz|drive\.google|dropbox\.com|mediafire\.com|catbox\.moe|pixeldrain|\.(zip|rar|7z)/i.test(contentStr) || /mega\.nz|drive\.google|dropbox\.com|mediafire\.com|catbox\.moe|pixeldrain|\.(zip|rar|7z)/i.test(embedStr);
  const hasArchiveTitle = /\b(pack|set|zip|rar|7z|dl|download|drive|mega|pdf|file|files|psd|clip|brush|brushes|model|blend)\b/i.test(titleStr);
  const hasArchiveTag = /\b(pack|set|zip|rar|7z|dl|download|drive|mega|pdf|files|psd|clip|brush|brushes|model|blend)\b/i.test(tagsStr);
  if (hasArchiveFile || hasArchiveLink || hasArchiveTitle || hasArchiveTag) {
    formats.push('archive');
  }

  // WIP (Work in Progress / Sketches / Drafts / Previews)
  const isWipTag = /\b(wip|w\.i\.p|w\/i\/p|work\s+in\s+progress|sketch|sketches|rough|draft|preview|doodle|doodles|lineart|line\s*art|progress|in\s+progress)\b/i.test(tagsStr);
  const isWipTitle = /\b(wip|w\.i\.p|w\/i\/p|work\s+in\s+progress|sketch|sketches|rough|draft|preview|doodle|doodles|lineart|line\s*art|in\s+progress)\b|[\[\(]wip[\]\)]|wip\s*#?\d+/i.test(titleStr);
  const isWipContent = /#(wip|sketch|workinprogress|draft|preview|doodle)\b|\[wip\]|\(wip\)/i.test(contentStr);
  if (isWipTag || isWipTitle || isWipContent) {
    formats.push('wip');
  }

  if (formats.length === 0) formats.push('text');
  return formats;
}

export function matchesPostFormat(post: Post, format: string): boolean {
  if (format === 'all') return true;
  return getPostFormats(post).includes(format);
}

export function isDirectMediaUrl(url: string): boolean {
  if (!url) return false;
  const clean = url.split('?')[0].split('#')[0].toLowerCase();
  if (/\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v|zip|rar|7z|tar|gz|pdf|mp3|wav|flac|opus|ogg|png|jpe?g|webp|gif|avif)$/i.test(clean)) {
    return true;
  }
  if (url.includes('.b-cdn.net/') && (url.includes('play_') || url.includes('.mp4'))) {
    return true;
  }
  return false;
}

export function extractDirectMediaLinks(raw: string): Array<{ url: string; name: string }> {
  if (!raw) return [];
  const results: Array<{ url: string; name: string }> = [];
  const seen = new Set<string>();

  // 1. Anchors: <a href="...">
  const anchorRegex = /<a\s+[^>]*href=["'](https?:\/\/[^"'>]+)["'][^>]*>(.*?)<\/a>/gi;
  let match: RegExpExecArray | null;
  while ((match = anchorRegex.exec(raw)) !== null) {
    const url = match[1];
    const text = match[2].replace(/<[^>]*>/g, '').trim();
    if (isDirectMediaUrl(url) && !seen.has(url)) {
      seen.add(url);
      const filename = text && !text.startsWith('http')
        ? text
        : decodeURIComponent(url.split('/').pop()?.split('?')[0] || 'Media File');
      results.push({ url, name: filename });
    }
  }

  // 2. Images: <img src="...">
  const imgRegex = /<img\s+[^>]*src=["'](https?:\/\/[^"'>]+)["'][^>]*>/gi;
  while ((match = imgRegex.exec(raw)) !== null) {
    const url = match[1];
    if (isDirectMediaUrl(url) && !seen.has(url)) {
      seen.add(url);
      const filename = decodeURIComponent(url.split('/').pop()?.split('?')[0] || 'Media File');
      results.push({ url, name: filename });
    }
  }

  // 3. Raw URLs in plain text
  const urlRegex = /https?:\/\/[^\s<>"')]+/gi;
  while ((match = urlRegex.exec(raw)) !== null) {
    const url = match[0];
    if (isDirectMediaUrl(url) && !seen.has(url)) {
      seen.add(url);
      const filename = decodeURIComponent(url.split('/').pop()?.split('?')[0] || 'Media File');
      results.push({ url, name: filename });
    }
  }

  return results;
}

export function extractCloudLinks(raw: string): string[] {
  if (!raw) return [];
  const regex = /https?:\/\/(?:[a-zA-Z0-9-]+\.)*(?:mega\.nz|mega\.co\.nz|pixeldrain\.com|dropbox\.com|drive\.google\.com|mediafire\.com|catbox\.moe|gofile\.io|iframely\.net|iframe\.ly)\/[^\s<>"')]+/gi;
  const matches = raw.match(regex) || [];
  return [...new Set(matches)];
}

export interface PostFileCounts {
  images: number;
  videos: number;
  audios: number;
  archives: number;
  documents: number;
  clouds: number;
  attachments: number;
  total: number;
}

export function getPostFileCounts(post: Post): PostFileCounts {
  const counts: PostFileCounts = {
    images: 0,
    videos: 0,
    audios: 0,
    archives: 0,
    documents: 0,
    clouds: 0,
    attachments: 0,
    total: 0
  };

  if (!post) return counts;

  const knownAttachments = post.attachment_count ?? 0;
  const hasLoadedAttachments = Array.isArray(post.attachments) && post.attachments.length > 0;

  if (!hasLoadedAttachments && knownAttachments > 0) {
    counts.attachments = knownAttachments;
    counts.total = knownAttachments;
    return counts;
  }

  const items: Attachment[] = [];
  const seenKeys = new Set<string>();

  const registerItem = (att?: Attachment | null) => {
    if (!att) return;
    const key = (
      att.path ||
      att.name ||
      (att.extra as any)?.storage_key ||
      (att as any)?.id ||
      ''
    ).toLowerCase();
    if (key) {
      if (seenKeys.has(key)) return;
      seenKeys.add(key);
    }
    items.push(att);
  };

  registerItem(post.file);
  if (Array.isArray(post.attachments)) {
    for (const a of post.attachments) {
      registerItem(a);
    }
  }

  for (const item of items) {
    const filename = (item.name || item.path || '').toLowerCase();
    if (isAttachmentVideo(item, item.path || item.name)) {
      counts.videos++;
    } else if (isAttachmentAudio(item, item.path || item.name)) {
      counts.audios++;
    } else if (/\.(zip|rar|7z|tar|gz|tar\.gz|tar\.xz|bz2|xz|cbz|cbr)(?:$|[?#])/i.test(filename)) {
      counts.archives++;
    } else if (/\.(pdf|epub|txt|doc|docx|psd|clip|blend|fbx|obj|stl)(?:$|[?#])/i.test(filename)) {
      counts.documents++;
    } else if (isAttachmentImage(item, item.path || item.name)) {
      counts.images++;
    } else {
      if ((item as any)?.preview_thumbhash || (item?.extra as any)?.preview_thumbhash) {
        counts.images++;
      } else {
        counts.documents++;
      }
    }
  }

  if (post.embed && typeof post.embed === 'object') {
    const embedStr = JSON.stringify(post.embed).toLowerCase();
    if (embedStr.includes('youtube') || embedStr.includes('vimeo') || embedStr.includes('redgifs') || embedStr.includes('video')) {
      if (counts.videos === 0) {
        counts.videos++;
      }
    } else if (embedStr.includes('soundcloud') || embedStr.includes('spotify') || embedStr.includes('bandcamp') || embedStr.includes('audio')) {
      if (counts.audios === 0) {
        counts.audios++;
      }
    }
  }

  const contentText = (post.content || post.substring || '') + ' ' + JSON.stringify(post.embed || {});
  const cloudLinks = extractCloudLinks(contentText);
  counts.clouds = cloudLinks.length;

  counts.total = counts.images + counts.videos + counts.audios + counts.archives + counts.documents + counts.clouds;
  return counts;
}

export interface DownloadTarget {
  mediaId: string;
  url: string;
  filename: string;
}

export function getPostDownloadTargets(post: Post): DownloadTarget[] {
  const targets: DownloadTarget[] = [];
  const items = [post.file, ...(post.attachments || [])].filter(Boolean) as Attachment[];
  const seenPaths = new Set<string>();

  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (!item.path) continue;
    const pathKey = item.path.toLowerCase();
    if (seenPaths.has(pathKey)) continue;
    seenPaths.add(pathKey);

    const url = attachmentMediaUrl(item, post.service);
    let filename = item.name || `media_${i + 1}`;
    if (!filename.includes('.')) {
      const ext = inferAttachmentExtension(item);
      filename = `${filename}.${ext}`;
    }

    targets.push({
      mediaId: item.path,
      url,
      filename
    });
  }

  const directMedia = extractDirectMediaLinks(post.content || post.substring || '');
  for (let i = 0; i < directMedia.length; i++) {
    const d = directMedia[i];
    if (!d.url) continue;
    const pathKey = d.url.toLowerCase();
    if (seenPaths.has(pathKey)) continue;
    seenPaths.add(pathKey);

    let filename = d.name || `embedded_media_${i + 1}`;
    if (!filename.includes('.')) {
      const ext = inferAttachmentExtension({ path: d.url, name: d.name });
      filename = `${filename}.${ext}`;
    }

    targets.push({
      mediaId: d.url,
      url: d.url,
      filename
    });
  }

  return targets;
}

export function getPlatformProfileUrl(service?: string, creatorId?: string, publicId?: string | number | null): string {
  if (!service || !creatorId) return '';
  const s = service.toLowerCase();
  const id = String(publicId || creatorId).trim();

  switch (s) {
    case 'patreon':
      return /^\d+$/.test(id) ? `https://www.patreon.com/user?u=${id}` : `https://www.patreon.com/${id}`;
    case 'fanbox':
      return /^\d+$/.test(id) ? `https://www.pixiv.net/fanbox/creator/${id}` : `https://${id}.fanbox.cc`;
    case 'fantia':
      return `https://fantia.jp/fanclubs/${id}`;
    case 'boosty':
      return `https://boosty.to/${id}`;
    case 'subscribestar':
      return `https://subscribestar.adult/${id}`;
    case 'gumroad':
      return `https://${id}.gumroad.com`;
    case 'onlyfans':
      return `https://onlyfans.com/${id}`;
    case 'fansly':
      return `https://fansly.com/${id}`;
    case 'candfans':
      return `https://candfans.jp/${id}`;
    case 'discord':
      return `https://discord.com/channels/${id}`;
    case 'afdian':
      return `https://afdian.com/a/${id}`;
    case 'cien':
    case 'ci-en':
      return `https://ci-en.dlsite.com/creator/${id}`;
    case 'dlsite':
      return `https://www.dlsite.com/maniax/circle/profile/=/maker_id/${id}`;
    default:
      return `https://${s}.com/${id}`;
  }
}

export function getFileExtension(filename?: string): string {
  if (!filename) return 'FILE';
  const clean = filename.split('?')[0].split('#')[0];
  const parts = clean.split('.');
  if (parts.length <= 1) return 'FILE';
  const ext = parts.pop()!;
  return ext.length > 6 ? 'FILE' : ext.toUpperCase();
}

export function getUnsupportedContainerFormat(filename?: string, url?: string): string | null {
  const target = (filename || url || '').split('?')[0].split('#')[0].toLowerCase();
  const match = target.match(/\.(avi|wmv|asf|flv|f4v|rmvb|rm|vob|divx|xvid|m2ts|ts)(?:$|[?#])/i);
  if (match) {
    return match[1].toUpperCase();
  }
  return null;
}

export function isH265Video(filename?: string, url?: string): boolean {
  if (!filename && !url) return false;
  const target = (filename || url || '').toLowerCase();
  return /\b(h265|hevc|x265)\b|\.(h265|hevc)(?:$|[?#])/i.test(target);
}

export type MediaErrorPreset =
  | 'unsupported_format'
  | 'unsupported_codec'
  | 'forbidden'
  | 'not_found'
  | 'rate_limited'
  | 'server_error'
  | 'unavailable'
  | 'unarchived'
  | 'network'
  | 'decode'
  | 'custom';

export interface MediaFailureState {
  preset: MediaErrorPreset;
  format?: string;
  message?: string;
  httpStatus?: number;
}

export function diagnoseVideoFailure(
  file?: Attachment | null,
  videoEl?: HTMLVideoElement | null,
  options?: { isLocal?: boolean; isUnarchived?: boolean }
): MediaFailureState {
  if (options?.isUnarchived) {
    return {
      preset: 'unarchived',
      message: videoEl?.error?.message || undefined
    };
  }
  const name = file?.name || '';
  const src = videoEl?.src || file?.path || '';
  const mediaErr = videoEl?.error;

  const isLocal = options?.isLocal ?? Boolean(
    ((src.startsWith('http://127.0.0.1') || src.startsWith('http://localhost')) && !src.includes('cloud_stream')) ||
    src.includes('asset.localhost') ||
    src.startsWith('file:') ||
    src.startsWith('tauri:') ||
    src.startsWith('asset:') ||
    (!src.startsWith('http://') && !src.startsWith('https://'))
  );

  const unsupported = getUnsupportedContainerFormat(name, src);
  if (unsupported) {
    return {
      preset: 'unsupported_format',
      format: unsupported,
      message: mediaErr?.message || undefined
    };
  }

  if (isH265Video(name, src)) {
    return {
      preset: 'unsupported_codec',
      format: 'H.265 / HEVC',
      message: mediaErr?.message || undefined
    };
  }

  if (mediaErr) {
    // 1: MEDIA_ERR_ABORTED, 2: MEDIA_ERR_NETWORK, 3: MEDIA_ERR_DECODE, 4: MEDIA_ERR_SRC_NOT_SUPPORTED
    if (mediaErr.code === 2) {
      return {
        preset: 'network',
        message: mediaErr.message || undefined
      };
    }
    if (mediaErr.code === 3) {
      return {
        preset: 'decode',
        message: mediaErr.message || undefined
      };
    }
    if (mediaErr.code === 4) {
      if (isLocal) {
        const ext = getFileExtension(name);
        return {
          preset: 'unsupported_format',
          format: ext !== 'FILE' ? ext : undefined,
          message: mediaErr.message || undefined
        };
      }
      // For remote web streams (MP4/WebM/HLS etc.), code 4 with empty/upstream error means 404/403/unavailable on source
      return {
        preset: 'unavailable',
        message: mediaErr.message || undefined
      };
    }
    if (mediaErr.message && mediaErr.message.trim().length > 0) {
      return {
        preset: 'custom',
        message: mediaErr.message
      };
    }
  }

  // Fallback for local files: if local video fails to play, it's decode or format
  if (isLocal) {
    return {
      preset: 'decode',
      message: mediaErr?.message || undefined
    };
  }

  // If container format is a known non-native web format fallback
  const ext = getFileExtension(name);
  if (['AVI', 'WMV', 'FLV', 'MKV', 'MOV', 'M4V', 'TS'].includes(ext)) {
    return {
      preset: 'unsupported_format',
      format: ext,
      message: mediaErr?.message || undefined
    };
  }

  return {
    preset: 'unavailable',
    message: mediaErr?.message || undefined
  };
}

export async function diagnoseVideoFailureAsync(
  file?: Attachment | null,
  videoEl?: HTMLVideoElement | null,
  options?: { isLocal?: boolean; isUnarchived?: boolean }
): Promise<MediaFailureState> {
  const syncDiag = diagnoseVideoFailure(file, videoEl, options);
  if (syncDiag.preset !== 'unavailable' && syncDiag.preset !== 'network') {
    return syncDiag;
  }

  const src = videoEl?.src || file?.path || '';
  if (src.startsWith('http://') || src.startsWith('https://')) {
    try {
      const resp = await fetch(src, { method: 'HEAD' });
      if (!resp.ok) {
        if (resp.status === 403) {
          return {
            preset: 'forbidden',
            httpStatus: 403,
            message: '403 Forbidden'
          };
        }
        if (resp.status === 404) {
          return {
            preset: 'not_found',
            httpStatus: 404,
            message: '404 Not Found'
          };
        }
        if (resp.status === 429) {
          return {
            preset: 'rate_limited',
            httpStatus: 429,
            message: '429 Too Many Requests'
          };
        }
        if (resp.status >= 500) {
          return {
            preset: 'server_error',
            httpStatus: resp.status,
            message: `HTTP ${resp.status} ${resp.statusText || 'Server Error'}`
          };
        }
      }
    } catch {
      // ignore network fetch failures
    }
  }

  return syncDiag;
}
