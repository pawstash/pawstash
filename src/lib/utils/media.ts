import type { PawchivePost, Attachment } from '$lib/types/pawchive';
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
    if (providers.length > 0) {
      const p = providers[0];
      if (kind === 'image') {
        if (p.image_url) return siteOrigin(p.image_url);
        if (configState.settings.image_domain) return siteOrigin(configState.settings.image_domain);
        return deriveSubdomainOrigin(p.api_url, 'image');
      }
      if (kind === 'file') {
        if (p.file_url) return siteOrigin(p.file_url);
        if (configState.settings.file_domain) return siteOrigin(configState.settings.file_domain);
        return deriveSubdomainOrigin(p.api_url, 'file');
      }
      if (p.api_url) return siteOrigin(p.api_url);
    }
  }

  const enabled = providerState.providers.filter((p) => p.enabled).sort((a, b) => a.priority - b.priority);
  if (enabled.length > 0) {
    const p = enabled[0];
    if (kind === 'image') {
      if (p.image_url) return siteOrigin(p.image_url);
      if (configState.settings.image_domain) return siteOrigin(configState.settings.image_domain);
      return deriveSubdomainOrigin(p.api_url, 'image');
    }
    if (kind === 'file') {
      if (p.file_url) return siteOrigin(p.file_url);
      if (configState.settings.file_domain) return siteOrigin(configState.settings.file_domain);
      return deriveSubdomainOrigin(p.api_url, 'file');
    }
    if (p.api_url) return siteOrigin(p.api_url);
  }

  const defaultApi = configState.settings.api_domain || 'pawchive.pw';
  if (kind === 'image') {
    return siteOrigin(configState.settings.image_domain || deriveSubdomainOrigin(defaultApi, 'image'));
  }
  if (kind === 'file') {
    return siteOrigin(configState.settings.file_domain || deriveSubdomainOrigin(defaultApi, 'file'));
  }
  return siteOrigin(defaultApi);
}

export function cleanMediaPath(rawPath: string): string {
  return rawPath.replace(/^\/*data\//, '').replace(/^\/+/, '');
}

export function creatorAvatarUrl(service: string, creatorId: string, thumbhash?: string | null): string {
  if (thumbhash) {
    const dataUrl = thumbHashToUrl(thumbhash);
    if (dataUrl) return dataUrl;
  }
  const s = (service || '').toLowerCase();
  if (isOnlyHavenService(s)) {
    return `https://img.cum.st/creator/${encodeURIComponent(s)}/${encodeURIComponent(creatorId)}/avatar.webp`;
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
    return `https://img.cum.st/creator/${encodeURIComponent(s)}/${encodeURIComponent(creatorId)}/header.webp`;
  }
  const origin = getProviderOrigin(service, 'api');
  return `${origin}/banners/${encodeURIComponent(s)}/${encodeURIComponent(creatorId)}`;
}

export function creatorPageUrl(service: string, creatorId: string): string {
  const origin = getProviderOrigin(service, 'api');
  return `${origin}/${encodeURIComponent(service.toLowerCase())}/user/${encodeURIComponent(creatorId)}`;
}

export function postPageUrl(service: string, creatorId: string, postId: string): string {
  const origin = getProviderOrigin(service, 'api');
  return `${origin}/${encodeURIComponent(service.toLowerCase())}/user/${encodeURIComponent(creatorId)}/post/${encodeURIComponent(postId)}`;
}

export function postMediaUrl(post: PawchivePost): string | null {
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

  const key = cleanMediaPath(file.path);
  const isHaven = isOnlyHavenService(service) || (file.extra as any)?.provider_id === 'coomer' || (file.extra as any)?.provider_id === 'onlyhaven';

  if (isHaven) {
    let ext = 'jpg';
    if (file.name && file.name.includes('.')) {
      ext = file.name.split('.').pop() || 'jpg';
    } else if ((file.extra as any)?.mime_type) {
      const mime = (file.extra as any).mime_type;
      if (mime.includes('video') || mime.includes('mp4')) ext = 'mp4';
      else if (mime.includes('png')) ext = 'png';
      else if (mime.includes('webp')) ext = 'webp';
      else if (mime.includes('gif')) ext = 'gif';
    } else if ((file.extra as any)?.kind === 'video') {
      ext = 'mp4';
    }
    return `https://e1.cum.st/media/${key}/original.${ext}`;
  }

  const origin = file.server ? resolveServerOrigin(file.server, service) : getProviderOrigin(service, 'file');
  return `${origin}/data/${key}`;
}

export function attachmentThumbnailUrl(file: Attachment, service: string): string {
  if (!file?.path) return '';
  if (file.path.startsWith('http://') || file.path.startsWith('https://') || file.path.startsWith('/cloud_stream/')) {
    return file.path;
  }

  const key = cleanMediaPath(file.path);
  const isHaven = isOnlyHavenService(service) || (file.extra as any)?.provider_id === 'coomer' || (file.extra as any)?.provider_id === 'onlyhaven';

  if (isHaven) {
    return `https://img.cum.st/thumbnail/${key}/preview.webp`;
  }

  const origin = getProviderOrigin(service, 'image');
  return `${origin}/thumbnail/data/${key}`;
}

export function postThumbnailUrl(post: PawchivePost): string | null {
  const media = post.file?.path ? post.file : post.attachments?.find((item) => item.path);
  const thumbhash = (media?.extra as any)?.preview_thumbhash || (post.file?.extra as any)?.preview_thumbhash || (post.attachments?.[0]?.extra as any)?.preview_thumbhash;

  if (thumbhash) {
    const dataUrl = thumbHashToUrl(thumbhash);
    if (dataUrl) return dataUrl;
  }

  if (!media?.path) return null;

  const key = cleanMediaPath(media.path);
  const isHaven = isOnlyHavenService(post.service) || (post.extra as any)?.provider_id === 'coomer' || (post.extra as any)?.provider_id === 'onlyhaven';

  if (isHaven) {
    return `https://img.cum.st/thumbnail/${key}/preview.webp`;
  }

  const origin = getProviderOrigin(post.service, 'image');
  return `${origin}/thumbnail/data/${key}`;
}

export function fancardMediaUrl(card: { hash?: string; ext?: string; mime?: string }, service: string): string {
  if (!card.hash || card.hash.length < 4) return '';
  const sub1 = card.hash.slice(0, 2);
  const sub2 = card.hash.slice(2, 4);
  let ext = (card.ext || '').replace(/^\.+/, '');
  if (!ext) {
    if (card.mime?.includes('png')) ext = 'png';
    else if (card.mime?.includes('webp')) ext = 'webp';
    else if (card.mime?.includes('gif')) ext = 'gif';
    else ext = 'jpg';
  }

  const isHaven = isOnlyHavenService(service);
  if (isHaven) {
    return `https://e1.cum.st/media/${sub1}/${sub2}/${card.hash}/original.${ext}`;
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
  let ext = (card.ext || '').replace(/^\.+/, '');
  if (!ext) {
    if (card.mime?.includes('png')) ext = 'png';
    else if (card.mime?.includes('webp')) ext = 'webp';
    else if (card.mime?.includes('gif')) ext = 'gif';
    else ext = 'jpg';
  }

  const isHaven = isOnlyHavenService(service);
  if (isHaven) {
    return `https://img.cum.st/thumbnail/${sub1}/${sub2}/${card.hash}/preview.webp`;
  }

  const origin = getProviderOrigin(service, 'image');
  return `${origin}/thumbnail/data/${sub1}/${sub2}/${card.hash}.${ext}`;
}

export function postAttachmentCount(post: PawchivePost): number {
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

export function getPostFormats(post: PawchivePost): string[] {
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

export function matchesPostFormat(post: PawchivePost, format: string): boolean {
  if (format === 'all') return true;
  return getPostFormats(post).includes(format);
}

export interface DownloadTarget {
  mediaId: string;
  url: string;
  filename: string;
}

export function getPostDownloadTargets(post: PawchivePost): DownloadTarget[] {
  const targets: DownloadTarget[] = [];
  const items = [post.file, ...(post.attachments || [])].filter(Boolean) as Attachment[];
  const seenPaths = new Set<string>();

  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (!item.path) continue;
    const pathKey = item.path.toLowerCase();
    if (seenPaths.has(pathKey)) continue;
    seenPaths.add(pathKey);

    const key = cleanMediaPath(item.path);
    const isHaven = isOnlyHavenService(post.service) || (post.extra as any)?.provider_id === 'coomer' || (post.extra as any)?.provider_id === 'onlyhaven';

    let url: string;
    let filename = item.name || `media_${i + 1}`;

    if (isHaven) {
      let ext = 'jpg';
      if (item.name && item.name.includes('.')) {
        ext = item.name.split('.').pop() || 'jpg';
      } else if ((item.extra as any)?.mime_type) {
        const mime = (item.extra as any).mime_type;
        if (mime.includes('video') || mime.includes('mp4')) ext = 'mp4';
        else if (mime.includes('png')) ext = 'png';
        else if (mime.includes('webp')) ext = 'webp';
        else if (mime.includes('gif')) ext = 'gif';
      } else if ((item.extra as any)?.kind === 'video') {
        ext = 'mp4';
      }
      url = `https://e1.cum.st/media/${key}/original.${ext}`;
      if (!filename.includes('.')) {
        filename = `${filename}.${ext}`;
      }
    } else {
      const origin = item.server ? resolveServerOrigin(item.server, post.service) : getProviderOrigin(post.service, 'file');
      url = `${origin}/data/${key}`;
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
