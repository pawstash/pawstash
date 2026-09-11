import type { Post, Attachment, Creator, CreatorProfile } from '$lib/types/content';
import { providerState } from '$lib/state/providerState.svelte';
import { thumbHashToUrl } from './thumbhash';
import { apiProbeDownloadSize } from '$lib/utils/ipc';
import { convertFileSrc } from '@tauri-apps/api/core';
import { serverPortState } from '$lib/state/serverPort.svelte';
import { configState } from '$lib/state/configState.svelte';

export function formatProviderName(name?: string): string {
  if (!name) return '';
  return name.replace(/\s*\([^)]*\)/g, '').trim();
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

export function resolveLocalMediaUrl(path?: string | null): string | undefined {
  if (!path) return undefined;
  if (path.startsWith('http://') || path.startsWith('https://')) return path;
  const port = serverPortState.port || 0;
  if (port > 0) {
    const norm = path.replace(/\\/g, '/');
    const clean = norm.startsWith('/') ? norm.slice(1) : norm;
    return serverPortState.mediaUrl(`/media/${encodeURI(clean)}`);
  }
  try {
    return convertFileSrc(path);
  } catch {
    return undefined;
  }
}

export function creatorAvatarUrl(_service: string, _creatorId: string, _thumbhash?: string | null, _explicitProviderId?: string): string {
  return '';
}

export function creatorAvatarSrc(creator?: Creator | CreatorProfile | null): string {
  if (!creator) return '';
  const c = creator as any;
  const avatarPath = typeof c.avatar_path === 'string' ? c.avatar_path : undefined;
  if (avatarPath) {
    const local = resolveLocalMediaUrl(avatarPath);
    if (local) return local;
  }
  const avatarUrl = typeof c.avatar_url === 'string' ? c.avatar_url : undefined;
  if (avatarUrl) {
    return avatarUrl;
  }
  return '';
}

export function creatorPlaceholderUrl(creator?: Creator | CreatorProfile | null): string | undefined {
  if (!creator || configState.settings.disable_blur_placeholders) return undefined;
  const thumb = (creator as any).avatar_thumbhash || (creator.extra as any)?.avatar_thumbhash;
  if (thumb && typeof thumb === 'string') {
    return thumbHashToUrl(thumb) || undefined;
  }
  return undefined;
}

export function creatorBannerUrl(_service: string, _creatorId: string, _thumbhash?: string | null, _explicitProviderId?: string): string {
  return '';
}

export function creatorBannerSrc(creator?: Creator | CreatorProfile | null): string {
  if (!creator) return '';
  const c = creator as any;
  const bannerPath = typeof c.banner_path === 'string' ? c.banner_path : undefined;
  if (bannerPath) {
    const local = resolveLocalMediaUrl(bannerPath);
    if (local) return local;
  }
  const bannerUrl = typeof c.banner_url === 'string' ? c.banner_url : undefined;
  if (bannerUrl) {
    return bannerUrl;
  }
  return '';
}

export function creatorBannerPlaceholderUrl(creator?: Creator | CreatorProfile | null): string | undefined {
  if (!creator || configState.settings.disable_blur_placeholders) return undefined;
  const thumb = (creator as any).banner_thumbhash || (creator.extra as any)?.banner_thumbhash;
  if (thumb && typeof thumb === 'string') {
    return thumbHashToUrl(thumb) || undefined;
  }
  return undefined;
}

export function creatorPageUrl(_service: string, _creatorId: string, _explicitProviderId?: string): string {
  return '';
}

export function postPageUrl(_service: string, _creatorId: string, _postId: string, _explicitProviderId?: string): string {
  return '';
}

export function postMediaUrl(post: Post): string | null {
  return post.media_url || null;
}

export function isPostUnarchived(post?: Post | null): boolean {
  if (!post) return false;
  const hasMedia = Boolean(
    (post.file && (post.file.path || post.file.name)) ||
    (post.attachments && post.attachments.length > 0) ||
    (post.attachment_count && post.attachment_count > 0)
  );
  if (!hasMedia) return false;

  if (post.has_full === false) return true;
  if (post.preview_state === 'pending') return true;

  if (post.attachments && Array.isArray(post.attachments)) {
    if (post.attachments.some((a: any) => a?.deferred === true || a?.extra?.deferred === true)) {
      return true;
    }
  }

  return false;
}

export function attachmentMediaUrl(file: Attachment, _service?: string, _post?: Post | null): string {
  if (!file) return '';
  if (file.url && (
    file.url.startsWith('http://') ||
    file.url.startsWith('https://') ||
    file.url.startsWith('/cloud_stream/')
  )) {
    return file.url;
  }
  if (file.path && (file.path.startsWith('http://') || file.path.startsWith('https://') || file.path.startsWith('/cloud_stream/'))) {
    return file.path;
  }

  return '';
}

export function attachmentThumbnailUrl(file: Attachment, _service?: string): string {
  if (!file) return '';
  return file.thumbnail_url || '';
}

export function postThumbnailUrl(post: Post): string | null {
  return post.thumbnail_url || null;
}

export function postThumbnailSrc(post?: Post | null): string | null {
  if (!post) return null;
  if (post.preview_path) {
    const local = resolveLocalMediaUrl(post.preview_path);
    if (local) return local;
  }
  if (post.thumbnail_url) {
    return post.thumbnail_url;
  }
  return postThumbnailUrl(post);
}

export function postPlaceholderUrl(post?: Post | null): string | undefined {
  if (!post || configState.settings.disable_blur_placeholders) return undefined;
  const media = post.file?.path ? post.file : post.attachments?.find((item) => item.path);
  const thumbhash = (media as any)?.preview_thumbhash ||
    (media?.extra as any)?.preview_thumbhash ||
    (post.file as any)?.preview_thumbhash ||
    (post.file?.extra as any)?.preview_thumbhash ||
    (post.attachments?.[0] as any)?.preview_thumbhash ||
    (post.attachments?.[0]?.extra as any)?.preview_thumbhash ||
    (post.extra as any)?.preview_thumbhash;

  if (thumbhash && typeof thumbhash === 'string') {
    return thumbHashToUrl(thumbhash) || undefined;
  }
  return undefined;
}

export function fancardMediaUrl(card: { media_url?: string; hash?: string; ext?: string }): string {
  return (card as any).media_url || '';
}

export function fancardThumbnailUrl(card: { thumbnail_url?: string; ihash?: string }): string {
  if ((card as any).thumbnail_url) return (card as any).thumbnail_url;
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

  const hasVideoFile = allFiles.some((f) => /\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(f));
  const hasVideoEmbed = /youtube|youtu\.be|vimeo|bilibili|streamable|gfycat|sproutvideo|vids\.io|redgifs|mediadelivery\.net|b-cdn\.net|\.(mp4|webm|mkv|mov|m4v)/i.test(embedStr) || /<video|\.(mp4|webm|mkv|mov|m4v)/i.test(contentStr);
  const hasVideoTitle = /\b(video|mp4|webm|movie|animation|anim|clip|mkv|mov|4k|1080p|720p|60fps|short|pv|trailer)\b/i.test(titleStr);
  const hasVideoTag = /\b(video|mp4|webm|movie|animation|anim|clip|mkv|mov|4k|1080p|720p|60fps|short|pv|trailer)\b/i.test(tagsStr);
  const hasDirectVideo = directMedia.some((d) => isVideoUrl(d.url) || isAttachmentVideo(null, d.url));
  if (hasVideoAttachment || hasVideoFile || hasVideoEmbed || hasVideoTitle || hasVideoTag || hasDirectVideo) {
    formats.push('video');
  }

  const hasImageFile = allFiles.some((f) => /\.(avif|bmp|gif|jpe?g|png|webp)(?:$|[?#])/i.test(f));
  const hasImageEmbed = /\.(avif|bmp|gif|jpe?g|png|webp)|<img/i.test(embedStr) || /<img/i.test(contentStr);
  const hasImageTag = /\b(photo|photos|image|images|pic|pics|picture|pictures|illustration|art|cg|drawing|wallpaper)\b/i.test(tagsStr);
  const hasDirectImage = directMedia.some((d) => isImageUrl(d.url) || isAttachmentImage(null, d.url));
  const isGenericMediaImage = (p.file?.path || p.file?.name) && !hasVideoAttachment && !hasAudioAttachment && !hasVideoFile;
  if (hasImageAttachment || hasImageFile || hasImageEmbed || hasImageTag || hasDirectImage || isGenericMediaImage) {
    formats.push('image');
  }

  const hasAudioFile = allFiles.some((f) => /\.(mp3|wav|ogg|m4a|flac|aac|opus|wma)(?:$|[?#])/i.test(f));
  const hasAudioEmbed = /soundcloud|bandcamp|spotify|audio|\.(mp3|wav|ogg|m4a|flac)/i.test(embedStr) || /<audio|\.(mp3|wav|ogg|m4a|flac)/i.test(contentStr);
  const hasAudioTitle = /\b(audio|mp3|wav|flac|sound|track|voice|podcast|asmr|song|music|ost)\b/i.test(titleStr);
  const hasAudioTag = /\b(audio|mp3|wav|flac|sound|track|voice|podcast|asmr|song|music|ost)\b/i.test(tagsStr);
  const hasDirectAudio = directMedia.some((d) => isAudioUrl(d.url) || isAttachmentAudio(null, d.url));
  if (hasAudioAttachment || hasAudioFile || hasAudioEmbed || hasAudioTitle || hasAudioTag || hasDirectAudio) {
    formats.push('audio');
  }

  const hasTextContent = Boolean(p.content && p.content.trim().length > 20);
  const isTextOnlyPost = (p.attachment_count ?? 0) === 0 && !p.file?.path && !p.file?.name && directMedia.length === 0;
  if (hasTextContent || isTextOnlyPost) {
    formats.push('text');
  }

  const hasArchiveFile = allFiles.some((f) => /\.(zip|rar|7z|tar|gz|pdf|txt|epub|html|cbz|cbr|psd|clip|blend|fbx|obj|stl)(?:$|[?#])/i.test(f));
  const hasArchiveLink = /mega\.nz|drive\.google|dropbox\.com|mediafire\.com|catbox\.moe|pixeldrain|\.(zip|rar|7z)/i.test(contentStr) || /mega\.nz|drive\.google|dropbox\.com|mediafire\.com|catbox\.moe|pixeldrain|\.(zip|rar|7z)/i.test(embedStr);
  const hasArchiveTitle = /\b(pack|set|zip|rar|7z|dl|download|drive|mega|pdf|file|files|psd|clip|brush|brushes|model|blend)\b/i.test(titleStr);
  const hasArchiveTag = /\b(pack|set|zip|rar|7z|dl|download|drive|mega|pdf|files|psd|clip|brush|brushes|model|blend)\b/i.test(tagsStr);
  if (hasArchiveFile || hasArchiveLink || hasArchiveTitle || hasArchiveTag) {
    formats.push('archive');
  }

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

  const isIgnoredMediaHost = (u: string) => {
    try {
      const parsed = new URL(u);
      const host = parsed.hostname.toLowerCase();
      return host.includes('patreonusercontent.com');
    } catch {
      return false;
    }
  };

  const anchorRegex = /<a\s+[^>]*href=["'](https?:\/\/[^"'>]+)["'][^>]*>(.*?)<\/a>/gi;
  let match: RegExpExecArray | null;
  while ((match = anchorRegex.exec(raw)) !== null) {
    let url = match[1].replace(/&amp;/g, '&').trim();
    const text = match[2].replace(/<[^>]*>/g, '').trim();
    if (isDirectMediaUrl(url) && !isIgnoredMediaHost(url) && !seen.has(url)) {
      seen.add(url);
      const filename = text && !text.startsWith('http')
        ? text
        : decodeURIComponent(url.split('/').pop()?.split('?')[0] || 'Media File');
      results.push({ url, name: filename });
    }
  }

  const plainText = raw.replace(/<[^>]+>/g, ' ');
  const urlRegex = /https?:\/\/[^\s<>"')]+/gi;
  while ((match = urlRegex.exec(plainText)) !== null) {
    let url = match[0].replace(/&amp;/g, '&').trim();
    while (url.endsWith('.') || url.endsWith(',') || url.endsWith(';') || url.endsWith(')')) {
      url = url.slice(0, -1);
    }
    if (isDirectMediaUrl(url) && !isIgnoredMediaHost(url) && !seen.has(url)) {
      seen.add(url);
      const filename = decodeURIComponent(url.split('/').pop()?.split('?')[0] || 'Media File');
      results.push({ url, name: filename });
    }
  }

  return results;
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

  counts.clouds = post.cloud_urls?.length || 0;

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

export function getPlatformProfileUrl(service?: string, creatorId?: string, publicId?: unknown): string {
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

export function getPlatformPostUrl(
  service?: string,
  creatorId?: string,
  postId?: string,
  publicId?: unknown
): string {
  if (!service || !postId) return '';
  const s = service.toLowerCase();
  const id = String(publicId || creatorId || '').trim();

  switch (s) {
    case 'patreon':
      return `https://www.patreon.com/posts/${postId}`;
    case 'fanbox':
      if (id && /^\d+$/.test(id)) {
        return `https://www.pixiv.net/fanbox/creator/${id}/post/${postId}`;
      }
      if (id) {
        return `https://${id}.fanbox.cc/posts/${postId}`;
      }
      return `https://www.fanbox.cc/posts/${postId}`;
    case 'fantia':
      return `https://fantia.jp/posts/${postId}`;
    case 'boosty':
      return id ? `https://boosty.to/${id}/posts/${postId}` : `https://boosty.to`;
    case 'subscribestar':
      return `https://subscribestar.adult/posts/${postId}`;
    case 'gumroad':
      return id ? `https://${id}.gumroad.com/p/${postId}` : `https://gumroad.com/l/${postId}`;
    case 'onlyfans':
      return id ? `https://onlyfans.com/${postId}/${id}` : `https://onlyfans.com/posts/${postId}`;
    case 'fansly':
      return `https://fansly.com/post/${postId}`;
    case 'candfans':
      return id ? `https://candfans.jp/${id}/posts/${postId}` : `https://candfans.jp/posts/${postId}`;
    case 'discord':
      return id ? `https://discord.com/channels/${id}/${postId}` : `https://discord.com`;
    case 'afdian':
      return `https://afdian.com/p/${postId}`;
    case 'cien':
    case 'ci-en':
      return id ? `https://ci-en.dlsite.com/creator/${id}/article/${postId}` : `https://ci-en.dlsite.com`;
    case 'dlsite':
      return `https://www.dlsite.com/maniax/work/=/product_id/${postId}.html`;
    default:
      return id ? `https://${s}.com/${id}/posts/${postId}` : `https://${s}.com`;
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
      // Browser error 4 is ambiguous remotely, so report it as source unavailability.
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

  if (isLocal) {
    return {
      preset: 'decode',
      message: mediaErr?.message || undefined
    };
  }

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

export function getAttachmentDeclaredSize(file?: Attachment | null): number {
  if (!file) return 0;
  if (typeof file.size === 'number' && file.size > 0) return file.size;
  if (typeof (file as any).filesize === 'number' && (file as any).filesize > 0) return (file as any).filesize;
  if (typeof (file as any).file_size === 'number' && (file as any).file_size > 0) return (file as any).file_size;
  if (typeof (file as any).bytes === 'number' && (file as any).bytes > 0) return (file as any).bytes;
  if (typeof file.size === 'string' && Number(file.size) > 0) return Number(file.size);
  return 0;
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
    let probeUrl = src;
    if (src.includes('/cloud_stream/proxy')) {
      try {
        const parsed = new URL(src);
        const target = parsed.searchParams.get('url');
        if (target) {
          probeUrl = target;
        }
      } catch {}
    }

    const declaredBytes = file ? getAttachmentDeclaredSize(file) : 0;

    try {
      const size = await apiProbeDownloadSize(probeUrl);
      if (typeof size === 'number' && size > 0) {
        return {
          preset: 'decode',
          message: 'Video codec or container is not supported by browser'
        };
      }
    } catch {}

    if (declaredBytes > 0) {
      return {
        preset: 'network',
        message: 'Network stream error while connecting to media source'
      };
    }
  }

  return syncDiag;
}
