import type { PawchivePost } from '$lib/types/pawchive';
import { configState } from '$lib/state/configState.svelte';

function siteOrigin(domain: string) {
  const value = domain.trim().replace(/\/+$/, '');
  return /^https?:\/\//i.test(value) ? value : `https://${value}`;
}

function creatorAssetUrl(kind: 'icons' | 'banners', service: string, creatorId: string) {
  const origin = siteOrigin(configState.settings.api_domain || 'pawchive.pw');
  return `${origin}/${kind}/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}`;
}

export function creatorAvatarUrl(service: string, creatorId: string) {
  return creatorAssetUrl('icons', service, creatorId);
}

export function creatorBannerUrl(service: string, creatorId: string) {
  return creatorAssetUrl('banners', service, creatorId);
}

export function creatorPageUrl(service: string, creatorId: string) {
  const origin = siteOrigin(configState.settings.api_domain || 'pawchive.pw');
  return `${origin}/${encodeURIComponent(service.toLowerCase())}/user/${encodeURIComponent(creatorId)}`;
}

export function postMediaUrl(post: PawchivePost) {
  const media = post.file?.path ? post.file : post.attachments?.find((item) => item.path);
  if (!media?.path) return null;
  const cdn = media.server || `https://${configState.settings.file_domain}`;
  return `${cdn}/data${media.path}`;
}

export function postThumbnailUrl(post: PawchivePost) {
  const media = post.file?.path ? post.file : post.attachments?.find((item) => item.path);
  if (!media?.path) return null;
  return `https://${configState.settings.image_domain}/thumbnail/data${media.path}`;
}

export function postAttachmentCount(post: PawchivePost) {
  return post.attachment_count ?? post.attachments?.length ?? 0;
}

export function isVideoUrl(url: string | null) {
  if (!url) return false;
  return /\.(mp4|webm|mkv|mov)(?:$|\?)/i.test(url);
}

export function isImageUrl(url: string | null) {
  if (!url) return false;
  return /\.(avif|bmp|gif|jpe?g|png|webp)(?:$|\?)/i.test(url);
}

export function getPostDownloadTargets(post: PawchivePost): { mediaId: string; url: string; filename: string }[] {
  const targets: { mediaId: string; url: string; filename: string }[] = [];
  const cdn = (server?: string) => server || `https://${configState.settings.file_domain}`;

  if (post.file?.path) {
    const filename = post.file.name || post.file.path.split('/').pop() || 'file';
    targets.push({
      mediaId: post.file.path,
      url: `${cdn(post.file.server)}/data${post.file.path}`,
      filename
    });
  }

  for (const attachment of post.attachments || []) {
    if (attachment?.path) {
      const path = attachment.path;
      const filename = attachment.name || path.split('/').pop() || 'attachment';
      if (!targets.some((t) => t.url.endsWith(path))) {
        targets.push({
          mediaId: path,
          url: `${cdn(attachment.server)}/data${path}`,
          filename
        });
      }
    }
  }

  return targets;
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

  const embedStr = JSON.stringify(post.embed || {}).toLowerCase();
  const contentStr = (post.content || '').toLowerCase();
  const titleStr = (post.title || '').toLowerCase();
  const formats: string[] = [];

  // Image
  const hasImageFile = allFiles.some((f) => /\.(avif|bmp|gif|jpe?g|png|webp)(?:$|\?)/i.test(f));
  const hasImageEmbed = /\.(avif|bmp|gif|jpe?g|png|webp)|<img/i.test(embedStr) || /<img/i.test(contentStr);
  if (hasImageFile || hasImageEmbed || Boolean(post.file?.path || post.file?.name)) {
    formats.push('image');
  }

  // Video
  const hasVideoFile = allFiles.some((f) => /\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)(?:$|\?)/i.test(f));
  const hasVideoEmbed = /youtube|vimeo|bilibili|streamable|gfycat|coomer|kemono|sproutvideo|vids\.io|\.(mp4|webm|mkv|mov|m4v)/i.test(embedStr) || /<video|\.(mp4|webm|mkv|mov|m4v)/i.test(contentStr);
  const hasVideoTitle = /\b(video|mp4|webm|movie|animation|anim|clip|mkv|mov|4k|1080p|720p|60fps|short|pv|trailer)\b/i.test(titleStr);
  if (hasVideoFile || hasVideoEmbed || hasVideoTitle) {
    formats.push('video');
  }

  // Audio
  const hasAudioFile = allFiles.some((f) => /\.(mp3|wav|ogg|m4a|flac|aac|opus|wma)(?:$|\?)/i.test(f));
  const hasAudioEmbed = /soundcloud|bandcamp|spotify|audio|\.(mp3|wav|ogg|m4a|flac)/i.test(embedStr) || /<audio|\.(mp3|wav|ogg|m4a|flac)/i.test(contentStr);
  const hasAudioTitle = /\b(audio|mp3|wav|flac|sound|track|voice|podcast|asmr|song|music|ost)\b/i.test(titleStr);
  if (hasAudioFile || hasAudioEmbed || hasAudioTitle) {
    formats.push('audio');
  }

  // Text
  const hasTextContent = Boolean(post.content && post.content.trim().length > 20);
  const isTextOnlyPost = (post.attachment_count ?? 0) === 0 && !post.file?.path;
  if (hasTextContent || isTextOnlyPost) {
    formats.push('text');
  }

  // Archive / Files
  const hasArchiveFile = allFiles.some((f) => /\.(zip|rar|7z|tar|gz|pdf|txt|epub|html|cbz|cbr|psd|clip)(?:$|\?)/i.test(f));
  const hasArchiveLink = /mega\.nz|drive\.google|dropbox\.com|mediafire\.com|catbox\.moe|pixeldrain|\.(zip|rar|7z)/i.test(contentStr) || /mega\.nz|drive\.google|dropbox\.com|mediafire\.com|catbox\.moe|pixeldrain|\.(zip|rar|7z)/i.test(embedStr);
  const hasArchiveTitle = /\b(pack|set|zip|rar|7z|dl|download|drive|mega|pdf|file|files|psd|clip|brush|brushes)\b/i.test(titleStr);
  if (hasArchiveFile || hasArchiveLink || hasArchiveTitle) {
    formats.push('archive');
  }

  // WIP (Work in Progress / Sketches / Drafts / Previews)
  const tagsStr = Array.isArray(post.tags)
    ? post.tags.join(' ').toLowerCase()
    : typeof post.tags === 'string'
      ? post.tags.toLowerCase()
      : '';
  const isWipTag = /\b(wip|w\.i\.p|w\/i\/p|work\s+in\s+progress|sketch|sketches|rough|draft|preview|doodle|doodles|lineart|line\s*art|progress|in\s+progress)\b/i.test(tagsStr);
  const isWipTitle = /\b(wip|w\.i\.p|w\/i\/p|work\s+in\s+progress|sketch|sketches|rough|draft|preview|doodle|doodles|lineart|line\s*art|in\s+progress)\b|[\[\(]wip[\]\)]|wip\s*#?\d+/i.test(titleStr);
  const isWipContent = /#(wip|sketch|workinprogress|draft|preview|doodle)\b|\[wip\]|\(wip\)/i.test(contentStr);
  if (isWipTag || isWipTitle || isWipContent) {
    formats.push('wip');
  }

  return formats;
}

export function matchesPostFormat(post: PawchivePost, fmt: string): boolean {
  return getPostFormats(post).includes(fmt);
}

