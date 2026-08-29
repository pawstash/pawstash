import type { Post, Attachment } from '$lib/types/content';
import { configState } from '$lib/state/configState.svelte';
import { providerState } from '$lib/state/providerState.svelte';
import { thumbHashToUrl } from './thumbhash';

function siteOrigin(domain: string): string {
  const value = domain.trim().replace(/\/+$/, '');
  return /^https?:\/\//i.test(value) ? value : `https://${value}`;
}

export function formatProviderName(name?: string): string {
  if (!name) return '';
  return name.replace(/Coomer/gi, 'OnlyHaven').replace(/\s*\([^)]*\)/g, '').trim();
}

export function isOnlyHavenService(service?: string): boolean {
  if (!service) return false;
  const s = service.toLowerCase();
  return s === 'onlyfans' || s === 'fansly' || s === 'candfans';
}

export function deriveSubdomainOrigin(baseUrl: string, kind: 'image' | 'file' | 'api'): string {
  const origin = siteOrigin(baseUrl);
  if (kind === 'api') return origin;

  try {
    const url = new URL(origin);
    const host = url.hostname;

    if (host.includes('cum.st') || host.includes('coomer')) {
      const parts = host.split('.');
      const baseHost = parts.length > 2 ? parts.slice(-2).join('.') : host;
      if (kind === 'image') return `${url.protocol}//img.${baseHost}`;
      if (kind === 'file') return `${url.protocol}//e1.${baseHost}`;
      return origin;
    }

    if (host.includes('kemono')) {
      const parts = host.split('.');
      const baseHost = parts.length > 2 ? parts.slice(-2).join('.') : host;
      if (kind === 'image') return `${url.protocol}//img.${baseHost}`;
      if (kind === 'file') return `${url.protocol}//c1.${baseHost}`;
      return origin;
    }

    const parts = host.split('.');
    const baseHost = parts.length > 2 ? parts.slice(-2).join('.') : host;
    const prefix = kind === 'image' ? 'img' : 'file';
    return `${url.protocol}//${prefix}.${baseHost}`;
  } catch {
    return origin;
  }
}

export function getProviderOrigin(service?: string, kind: 'api' | 'image' | 'file' = 'api'): string {
  if (service) {
    if (isOnlyHavenService(service)) {
      const havenProviders = providerState.getProvidersForService(service);
      const havenBase = havenProviders[0]?.api_url || 'https://cum.st';
      return deriveSubdomainOrigin(havenBase, kind);
    }
    const providers = providerState.getProvidersForService(service);
    if (providers && providers.length > 0) {
      const p = providers[0];
      if (kind === 'image') {
        if (p.image_url) return siteOrigin(p.image_url);
        if (configState.settings.image_domain && !configState.settings.image_domain.startsWith('pawchive.pw')) {
          return siteOrigin(configState.settings.image_domain);
        }
        return deriveSubdomainOrigin(p.api_url || 'pawchive.pw', 'image');
      }
      if (kind === 'file') {
        if (p.file_url) return siteOrigin(p.file_url);
        if (configState.settings.file_domain && !configState.settings.file_domain.startsWith('pawchive.pw')) {
          return siteOrigin(configState.settings.file_domain);
        }
        return deriveSubdomainOrigin(p.api_url || 'pawchive.pw', 'file');
      }
      if (p.api_url) return siteOrigin(p.api_url);
    }
  }

  const enabled = providerState.providers.filter((p) => p.enabled).sort((a, b) => a.priority - b.priority);
  if (enabled.length > 0) {
    const p = enabled[0];
    if (kind === 'image') {
      if (p.image_url) return siteOrigin(p.image_url);
      return deriveSubdomainOrigin(p.api_url || 'pawchive.pw', 'image');
    }
    if (kind === 'file') {
      if (p.file_url) return siteOrigin(p.file_url);
      return deriveSubdomainOrigin(p.api_url || 'pawchive.pw', 'file');
    }
    if (p.api_url) return siteOrigin(p.api_url);
  }

  const defaultApi = configState.settings.api_domain || 'pawchive.pw';
  if (kind === 'image') {
    return deriveSubdomainOrigin(configState.settings.image_domain || defaultApi, 'image');
  }
  if (kind === 'file') {
    return deriveSubdomainOrigin(configState.settings.file_domain || defaultApi, 'file');
  }
  return siteOrigin(defaultApi);
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

export function inferAttachmentExtension(file?: { name?: string; path?: string; extra?: any } | null, fallback = 'jpg'): string {
  if (!file) return fallback;

  const nameExt = file.name?.split('.').pop()?.trim().toLowerCase();
  if (nameExt && nameExt.length <= 8) return nameExt;

  const pathExt = file.path?.split('.').pop()?.split(/[?#]/)[0].trim().toLowerCase();
  if (pathExt && pathExt.length <= 8 && !pathExt.includes('/')) return pathExt;

  const mime = String((file.extra as any)?.mime_type || '').trim().toLowerCase();
  if (mime) {
    if (MIME_MAP[mime]) return MIME_MAP[mime];
    const sub = mime.split('/')[1]?.split(';')[0]?.replace(/^x-/, '').trim();
    if (sub && sub.length <= 8 && sub !== 'octet-stream') return sub;
  }

  const kind = (file.extra as any)?.kind;
  if (kind === 'video') return 'mp4';
  if (kind === 'audio') return 'mp3';
  if (kind === 'archive') return 'zip';
  if (kind === 'document') return 'pdf';

  return fallback;
}

export function creatorAvatarUrl(service: string, creatorId: string, thumbhash?: string | null): string {
  if (thumbhash) {
    const dataUrl = thumbHashToUrl(thumbhash);
    if (dataUrl) return dataUrl;
  }
  const s = (service || '').toLowerCase();
  if (isOnlyHavenService(s)) {
    const origin = getProviderOrigin(service, 'image');
    return `${origin}/creator/${encodeURIComponent(s)}/${encodeURIComponent(creatorId)}/avatar.webp`;
  }
  const origin = getProviderOrigin(service, 'api');
  return `${origin}/icons/${encodeURIComponent(s)}/${encodeURIComponent(creatorId)}`;
}

export function creatorBannerUrl(service: string, creatorId: string, thumbhash?: string | null): string {
  if (thumbhash) {
    const dataUrl = thumbHashToUrl(thumbhash);
    if (dataUrl) return dataUrl;
  }
  const s = (service || '').toLowerCase();
  if (isOnlyHavenService(s)) {
    const origin = getProviderOrigin(service, 'image');
    return `${origin}/creator/${encodeURIComponent(s)}/${encodeURIComponent(creatorId)}/header.webp`;
  }
  const origin = getProviderOrigin(service, 'api');
  return `${origin}/banners/${encodeURIComponent(s)}/${encodeURIComponent(creatorId)}`;
}

export function creatorPageUrl(service: string, creatorId: string): string {
  const origin = getProviderOrigin(service, 'api');
  if (isOnlyHavenService(service)) {
    return `${origin}/creators/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}`;
  }
  return `${origin}/${encodeURIComponent(service.toLowerCase())}/user/${encodeURIComponent(creatorId)}`;
}

export function postPageUrl(service: string, creatorId: string, postId: string): string {
  const origin = getProviderOrigin(service, 'api');
  if (isOnlyHavenService(service)) {
    return `${origin}/creators/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}/post/${encodeURIComponent(postId)}`;
  }
  return `${origin}/${encodeURIComponent(service.toLowerCase())}/user/${encodeURIComponent(creatorId)}/post/${encodeURIComponent(postId)}`;
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

  const providerOrigin = getProviderOrigin(service, 'file');
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

  const key = cleanMediaPath(file.path);
  const isHaven = isOnlyHavenService(service) || (file.extra as any)?.provider_id === 'coomer' || (file.extra as any)?.provider_id === 'onlyhaven';

  if (isHaven) {
    const ext = inferAttachmentExtension(file);
    const origin = getProviderOrigin(service, 'file');
    return `${origin}/media/${key}/original.${ext}`;
  }

  if (file.server) {
    const origin = resolveServerOrigin(file.server, service);
    return `${origin}/data/${key}`;
  }

  const origin = getProviderOrigin(service, 'file');
  return `${origin}/data/${key}`;
}

export function attachmentThumbnailUrl(file: Attachment, service: string): string {
  if (!file?.path) return '';
  if (isAttachmentVideo(file, file.path)) {
    const thumb = (file.extra as any)?.thumbnail || (file.extra as any)?.preview;
    if (thumb && typeof thumb === 'string') return thumb;
    return '';
  }
  if (file.path.startsWith('/cloud_stream/')) {
    return '';
  }
  if (file.path.startsWith('http://') || file.path.startsWith('https://')) {
    return file.path;
  }

  const key = cleanMediaPath(file.path);
  const isHaven = isOnlyHavenService(service) || (file.extra as any)?.provider_id === 'coomer' || (file.extra as any)?.provider_id === 'onlyhaven';
  const origin = getProviderOrigin(service, 'image');

  if (isHaven) {
    return `${origin}/thumbnail/${key}/preview.webp`;
  }

  return `${origin}/thumbnail/data/${key}`;
}

export function postThumbnailUrl(post: Post): string | null {
  const media = post.file?.path ? post.file : post.attachments?.find((item) => item.path);
  const thumbhash = (media?.extra as any)?.preview_thumbhash || (post.file?.extra as any)?.preview_thumbhash || (post.attachments?.[0]?.extra as any)?.preview_thumbhash;

  if (thumbhash) {
    const dataUrl = thumbHashToUrl(thumbhash);
    if (dataUrl) return dataUrl;
  }

  if (!media?.path) return null;

  const key = cleanMediaPath(media.path);
  const isHaven = isOnlyHavenService(post.service) || (post.extra as any)?.provider_id === 'coomer' || (post.extra as any)?.provider_id === 'onlyhaven';
  const origin = getProviderOrigin(post.service, 'image');

  if (isHaven) {
    return `${origin}/thumbnail/${key}/preview.webp`;
  }

  return `${origin}/thumbnail/data/${key}`;
}

export function fancardMediaUrl(card: { hash?: string; ext?: string; mime?: string }, service: string): string {
  if (!card.hash || card.hash.length < 4) return '';
  const sub1 = card.hash.slice(0, 2);
  const sub2 = card.hash.slice(2, 4);
  const ext = (card.ext || '').replace(/^\.+/, '') || (card.mime?.includes('png') ? 'png' : card.mime?.includes('webp') ? 'webp' : card.mime?.includes('gif') ? 'gif' : 'jpg');

  const isHaven = isOnlyHavenService(service);
  if (isHaven) {
    const origin = getProviderOrigin(service, 'file');
    return `${origin}/media/${sub1}/${sub2}/${card.hash}/original.${ext}`;
  }

  const origin = getProviderOrigin(service, 'image');
  return `${origin}/data/${sub1}/${sub2}/${card.hash}.${ext}`;
}

export function fancardThumbnailUrl(card: { hash?: string; ext?: string; mime?: string; ihash?: string }, service: string): string {
  if (card.ihash) {
    const dataUrl = thumbHashToUrl(card.ihash);
    if (dataUrl) return dataUrl;
  }
  if (!card.hash || card.hash.length < 4) return '';
  const sub1 = card.hash.slice(0, 2);
  const sub2 = card.hash.slice(2, 4);
  const ext = (card.ext || '').replace(/^\.+/, '') || (card.mime?.includes('png') ? 'png' : card.mime?.includes('webp') ? 'webp' : card.mime?.includes('gif') ? 'gif' : 'jpg');

  const origin = getProviderOrigin(service, 'image');
  if (isOnlyHavenService(service)) {
    return `${origin}/thumbnail/${sub1}/${sub2}/${card.hash}/preview.webp`;
  }

  return `${origin}/thumbnail/data/${sub1}/${sub2}/${card.hash}.${ext}`;
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
  const kind = ((file?.extra as any)?.kind || '').toLowerCase();
  if (kind === 'video') return true;
  const mime = ((file?.extra as any)?.mime_type || '').toLowerCase();
  if (mime.includes('video') || mime.includes('mp4')) return true;
  return false;
}

export function isAttachmentImage(file?: Attachment | null, url?: string | null): boolean {
  if (!file && !url) return false;
  if (url && isImageUrl(url)) return true;
  const name = (file?.name || '').toLowerCase();
  if (/\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(name)) return true;
  const kind = ((file?.extra as any)?.kind || '').toLowerCase();
  if (kind === 'image' || kind === 'gif') return true;
  const mime = ((file?.extra as any)?.mime_type || '').toLowerCase();
  if (mime.includes('image')) return true;
  return false;
}

export function isAttachmentAudio(file?: Attachment | null, url?: string | null): boolean {
  if (!file && !url) return false;
  if (url && isAudioUrl(url)) return true;
  const name = (file?.name || '').toLowerCase();
  if (/\.(mp3|m4a|aac|wav|ogg|opus|flac)(?:$|[?#])/i.test(name)) return true;
  const kind = ((file?.extra as any)?.kind || '').toLowerCase();
  if (kind === 'audio') return true;
  const mime = ((file?.extra as any)?.mime_type || '').toLowerCase();
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
  const allFiles: string[] = [];
  if (post.file?.path) allFiles.push(post.file.path.toLowerCase());
  if (post.file?.name) allFiles.push(post.file.name.toLowerCase());
  if (post.attachments && Array.isArray(post.attachments)) {
    for (const att of post.attachments) {
      if (att?.path) allFiles.push(att.path.toLowerCase());
      if (att?.name) allFiles.push(att.name.toLowerCase());
    }
  }

  const items = [post.file, ...(post.attachments || [])].filter(Boolean) as Attachment[];
  const hasVideoAttachment = items.some((it) => isAttachmentVideo(it, it.path));
  const hasImageAttachment = items.some((it) => isAttachmentImage(it, it.path));

  const embedStr = JSON.stringify(post.embed || {}).toLowerCase();
  const contentStr = (post.content || '').toLowerCase();
  const titleStr = (post.title || '').toLowerCase();
  const tagsStr = Array.isArray(post.tags)
    ? post.tags.join(' ').toLowerCase()
    : typeof post.tags === 'string'
      ? post.tags.toLowerCase()
      : '';

  const formats: string[] = [];

  // Video
  const hasVideoFile = allFiles.some((f) => /\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(f));
  const hasVideoEmbed = /youtube|youtu\.be|vimeo|bilibili|streamable|gfycat|coomer|kemono|sproutvideo|vids\.io|redgifs|\.(mp4|webm|mkv|mov|m4v)/i.test(embedStr) || /<video|\.(mp4|webm|mkv|mov|m4v)/i.test(contentStr);
  const hasVideoTitle = /\b(video|mp4|webm|movie|animation|anim|clip|mkv|mov|4k|1080p|720p|60fps|short|pv|trailer)\b/i.test(titleStr);
  if (hasVideoAttachment || hasVideoFile || hasVideoEmbed || hasVideoTitle) {
    formats.push('video');
  }

  // Image
  const hasImageFile = allFiles.some((f) => /\.(avif|bmp|gif|jpe?g|png|webp)(?:$|[?#])/i.test(f));
  const hasImageEmbed = /\.(avif|bmp|gif|jpe?g|png|webp)|<img/i.test(embedStr) || /<img/i.test(contentStr);
  if (hasImageAttachment || hasImageFile || hasImageEmbed || Boolean(post.file?.path || post.file?.name)) {
    formats.push('image');
  }

  // Audio
  const hasAudioFile = allFiles.some((f) => /\.(mp3|wav|ogg|m4a|flac|aac|opus|wma)(?:$|[?#])/i.test(f));
  const hasAudioEmbed = /soundcloud|bandcamp|spotify|audio|\.(mp3|wav|ogg|m4a|flac)/i.test(embedStr) || /<audio|\.(mp3|wav|ogg|m4a|flac)/i.test(contentStr);
  const hasAudioTitle = /\b(audio|mp3|wav|flac|sound|track|voice|podcast|asmr|song|music|ost)\b/i.test(titleStr);
  if (hasAudioFile || hasAudioEmbed || hasAudioTitle) {
    formats.push('audio');
  }

  // Text
  const hasTextContent = Boolean(post.content && post.content.trim().length > 20);
  const isTextOnlyPost = (post.attachment_count ?? 0) === 0 && !post.file?.path && !post.file?.name;
  if (hasTextContent || isTextOnlyPost) {
    formats.push('text');
  }

  // Archive / Files
  const hasArchiveFile = allFiles.some((f) => /\.(zip|rar|7z|tar|gz|pdf|txt|epub|html|cbz|cbr|psd|clip|blend|fbx|obj|stl)(?:$|[?#])/i.test(f));
  const hasArchiveLink = /mega\.nz|drive\.google|dropbox\.com|mediafire\.com|catbox\.moe|pixeldrain|\.(zip|rar|7z)/i.test(contentStr) || /mega\.nz|drive\.google|dropbox\.com|mediafire\.com|catbox\.moe|pixeldrain|\.(zip|rar|7z)/i.test(embedStr);
  const hasArchiveTitle = /\b(pack|set|zip|rar|7z|dl|download|drive|mega|pdf|file|files|psd|clip|brush|brushes|model|blend)\b/i.test(titleStr);
  if (hasArchiveFile || hasArchiveLink || hasArchiveTitle) {
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

export type MediaErrorPreset = 'unsupported_format' | 'unsupported_codec' | 'unavailable' | 'unarchived' | 'network' | 'decode' | 'custom';

export interface MediaFailureState {
  preset: MediaErrorPreset;
  format?: string;
  message?: string;
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
