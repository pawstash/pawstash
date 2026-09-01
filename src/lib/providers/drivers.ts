import type { ProviderConfig } from '$lib/types/provider';

export interface ProviderDriver {
  resolveMediaUrl(config: ProviderConfig, path: string, server?: string, ext?: string): string;
  resolveThumbnailUrl(config: ProviderConfig, path: string): string;
  resolveAvatarUrl(config: ProviderConfig, service: string, creatorId: string): string;
  resolveBannerUrl(config: ProviderConfig, service: string, creatorId: string): string;
  resolveCreatorPageUrl(config: ProviderConfig, service: string, creatorId: string): string;
  resolvePostPageUrl(config: ProviderConfig, service: string, creatorId: string, postId: string): string;
  resolveFancardMediaUrl(config: ProviderConfig, service: string, card: { hash?: string; ext?: string }): string;
  resolveFancardThumbnailUrl(config: ProviderConfig, service: string, card: { hash?: string; ext?: string }): string;
}

function siteOrigin(domain: string): string {
  const value = (domain || '').trim().replace(/\/+$/, '');
  return /^https?:\/\//i.test(value) ? value : `https://${value}`;
}

export function deriveSubdomainOrigin(baseUrl: string, prefix: string): string {
  const origin = siteOrigin(baseUrl);
  try {
    const url = new URL(origin);
    const host = url.hostname;
    const baseHost = host.trimStart().replace(/^(www\.|api\.)/i, '');
    const parts = baseHost.split('.');
    const domain = parts.length > 2 ? parts.slice(-2).join('.') : baseHost;
    return `${url.protocol}//${prefix}.${domain}`;
  } catch {
    return origin;
  }
}

export const PawchiveDriver: ProviderDriver = {
  resolveMediaUrl(config, path, server) {
    const clean = path.replace(/^\/*data\//, '').replace(/^\/+/, '');
    if (server) {
      const srv = server.trim().replace(/\/+$/, '');
      if (/^https?:\/\//i.test(srv)) return `${srv}/data/${clean}`;
      if (srv.includes('.')) return `https://${srv}/data/${clean}`;
      const fileOrigin = config.file_url ? siteOrigin(config.file_url) : deriveSubdomainOrigin(config.api_url, 'file');
      try {
        const url = new URL(fileOrigin);
        const parts = url.hostname.split('.');
        const base = parts.length > 2 ? parts.slice(1).join('.') : url.hostname;
        return `${url.protocol}//${srv}.${base}/data/${clean}`;
      } catch {
        return `${fileOrigin}/data/${clean}`;
      }
    }
    const origin = config.file_url ? siteOrigin(config.file_url) : deriveSubdomainOrigin(config.api_url, 'file');
    return `${origin}/data/${clean}`;
  },

  resolveThumbnailUrl(config, path) {
    const clean = path.replace(/^\/*data\//, '').replace(/^\/+/, '');
    const origin = config.image_url ? siteOrigin(config.image_url) : deriveSubdomainOrigin(config.api_url, 'img');
    return `${origin}/thumbnail/data/${clean}`;
  },

  resolveAvatarUrl(config, service, creatorId) {
    const origin = siteOrigin(config.api_url);
    return `${origin}/icons/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}`;
  },

  resolveBannerUrl(config, service, creatorId) {
    const origin = siteOrigin(config.api_url);
    return `${origin}/banners/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}`;
  },

  resolveCreatorPageUrl(config, service, creatorId) {
    const origin = siteOrigin(config.api_url);
    return `${origin}/${encodeURIComponent(service.toLowerCase())}/user/${encodeURIComponent(creatorId)}`;
  },

  resolvePostPageUrl(config, service, creatorId, postId) {
    const origin = siteOrigin(config.api_url);
    return `${origin}/${encodeURIComponent(service.toLowerCase())}/user/${encodeURIComponent(creatorId)}/post/${encodeURIComponent(postId)}`;
  },

  resolveFancardMediaUrl(config, service, card) {
    const hash = card.hash || '';
    const sub1 = hash.slice(0, 2);
    const sub2 = hash.slice(2, 4);
    const ext = card.ext || 'jpg';
    const origin = config.image_url ? siteOrigin(config.image_url) : deriveSubdomainOrigin(config.api_url, 'img');
    return `${origin}/data/${sub1}/${sub2}/${hash}.${ext}`;
  },

  resolveFancardThumbnailUrl(config, service, card) {
    const hash = card.hash || '';
    const sub1 = hash.slice(0, 2);
    const sub2 = hash.slice(2, 4);
    const ext = card.ext || 'jpg';
    const origin = config.image_url ? siteOrigin(config.image_url) : deriveSubdomainOrigin(config.api_url, 'img');
    return `${origin}/thumbnail/data/${sub1}/${sub2}/${hash}.${ext}`;
  }
};

export const OnlyHavenDriver: ProviderDriver = {
  resolveMediaUrl(config, path, _server, ext) {
    const clean = path.replace(/^\/*media\//, '').replace(/^\/*data\//, '').replace(/^\/+/, '');
    const origin = config.file_url ? siteOrigin(config.file_url) : deriveSubdomainOrigin(config.api_url, 'e1');
    if (clean.includes('.')) {
      return `${origin}/media/${clean}`;
    }
    const finalExt = ext || 'jpg';
    return `${origin}/media/${clean}/original.${finalExt}`;
  },

  resolveThumbnailUrl(config, path) {
    const clean = path.replace(/^\/*media\//, '').replace(/^\/*data\//, '').replace(/^\/+/, '');
    const origin = config.image_url ? siteOrigin(config.image_url) : deriveSubdomainOrigin(config.api_url, 'img');
    return `${origin}/thumbnail/${clean}/preview.webp`;
  },

  resolveAvatarUrl(config, service, creatorId) {
    const origin = config.image_url ? siteOrigin(config.image_url) : deriveSubdomainOrigin(config.api_url, 'img');
    return `${origin}/creator/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}/avatar.webp`;
  },

  resolveBannerUrl(config, service, creatorId) {
    const origin = config.image_url ? siteOrigin(config.image_url) : deriveSubdomainOrigin(config.api_url, 'img');
    return `${origin}/creator/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}/header.webp`;
  },

  resolveCreatorPageUrl(config, service, creatorId) {
    const origin = siteOrigin(config.api_url);
    return `${origin}/creators/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}`;
  },

  resolvePostPageUrl(config, service, creatorId, postId) {
    const origin = siteOrigin(config.api_url);
    return `${origin}/creators/${encodeURIComponent(service.toLowerCase())}/${encodeURIComponent(creatorId)}/post/${encodeURIComponent(postId)}`;
  },

  resolveFancardMediaUrl(config, service, card) {
    const hash = card.hash || '';
    const sub1 = hash.slice(0, 2);
    const sub2 = hash.slice(2, 4);
    const ext = card.ext || 'jpg';
    const origin = config.file_url ? siteOrigin(config.file_url) : deriveSubdomainOrigin(config.api_url, 'e1');
    return `${origin}/media/${sub1}/${sub2}/${hash}/original.${ext}`;
  },

  resolveFancardThumbnailUrl(config, service, card) {
    const hash = card.hash || '';
    const sub1 = hash.slice(0, 2);
    const sub2 = hash.slice(2, 4);
    const origin = config.image_url ? siteOrigin(config.image_url) : deriveSubdomainOrigin(config.api_url, 'img');
    return `${origin}/thumbnail/${sub1}/${sub2}/${hash}/preview.webp`;
  }
};

const DRIVERS: Record<string, ProviderDriver> = {
  pawchive: PawchiveDriver,
  onlyhaven: OnlyHavenDriver,
};

export function getProviderDriver(providerId?: string): ProviderDriver {
  if (!providerId) return PawchiveDriver;
  return DRIVERS[providerId.toLowerCase()] || PawchiveDriver;
}
