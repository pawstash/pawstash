<script module lang="ts">
  import type { ResolvedPostLink } from '$lib/types/content';
  import { apiResolveExternalPostLink } from '$lib/utils/ipc';

  const ALLOWED_TAGS = new Set([
    'a', 'b', 'blockquote', 'br', 'code', 'del', 'div', 'em', 'figcaption', 'figure',
    'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'hr', 'i', 'iframe', 'img', 'li', 'ol', 'p', 'pre',
    's', 'span', 'strong', 'sub', 'sup', 'table', 'tbody', 'td', 'th', 'thead', 'tr',
    'u', 'ul'
  ]);
  const DROP_TAGS = new Set(['base', 'button', 'embed', 'form', 'input', 'link', 'math', 'meta', 'object', 'script', 'style', 'svg', 'textarea']);
  const resolutionCache = new Map<string, Promise<ResolvedPostLink | null>>();

  export function isSafeIframeSrc(url: string): boolean {
    try {
      const u = new URL(url);
      if (!['http:', 'https:'].includes(u.protocol)) return false;
      const host = u.hostname.toLowerCase();
      return (
        host.includes('google.com') ||
        host.includes('docs.google.com') ||
        host.includes('forms.gle') ||
        host.includes('strawpoll.com') ||
        host.includes('strawpoll.me') ||
        host.includes('youtube.com') ||
        host.includes('youtube-nocookie.com') ||
        host.includes('youtu.be') ||
        host.includes('player.vimeo.com') ||
        host.includes('vimeo.com') ||
        host.includes('soundcloud.com') ||
        host.includes('spotify.com') ||
        host.includes('bilibili.com') ||
        host.includes('nicovideo.jp') ||
        host.includes('iframely.net') ||
        host.includes('iframe.ly') ||
        host.includes('mega.nz') ||
        host.includes('pawchive.pw') ||
        host.includes('kemono.party') ||
        host.includes('kemono.su') ||
        host.includes('coomer.party') ||
        host.includes('coomer.su')
      );
    } catch {
      return false;
    }
  }

  export function smartLinkPlatform(raw: string): string | null {
    try {
      const url = new URL(raw);
      if (!['http:', 'https:'].includes(url.protocol)) return null;
      const host = url.hostname.replace(/^www\./, '').toLocaleLowerCase();
      if (host === 'patreon.com' || host.endsWith('.patreon.com')) return 'patreon';
      if (host === 'fanbox.cc' || host.endsWith('.fanbox.cc')) return 'fanbox';
      if (host === 'fantia.jp' || host.endsWith('.fantia.jp')) return 'fantia';
      if (host === 'subscribestar.com' || host.endsWith('.subscribestar.com') || host === 'subscribestar.adult' || host.endsWith('.subscribestar.adult')) return 'subscribestar';
      if (host === 'boosty.to' || host.endsWith('.boosty.to')) return 'boosty';
      if (host === 'afdian.com' || host.endsWith('.afdian.com') || host === 'afdian.net' || host.endsWith('.afdian.net')) return 'afdian';
      if (host === 'onlyfans.com' || host.endsWith('.onlyfans.com')) return 'onlyfans';
      if (host === 'fansly.com' || host.endsWith('.fansly.com')) return 'fansly';
      if (host === 'candfans.jp' || host.endsWith('.candfans.jp')) return 'candfans';
      if (host.includes('proton.me') || host.includes('protondrive.com')) return 'proton';
      if (host.includes('bunny.net') || host.includes('mediadelivery.net') || host.includes('b-cdn.net')) return 'bunny';
      if (host.includes('gofile.io')) return 'gofile';
      if (host.includes('mediafire.com')) return 'mediafire';
      if (host.includes('terabox.com') || host.includes('1024tera.com') || host.includes('teraboxapp.com')) return 'terabox';
      if (host.includes('catbox.moe') || host.includes('files.catbox.moe')) return 'catbox';
      if (host.includes('workupload.com')) return 'workupload';
      if (host.includes('qiwi.gg')) return 'qiwi';
      if (host.includes('send.cm')) return 'sendcm';
      if (host.includes('kemono') || host.includes('pawchive')) return 'pawchive';
      if (host.includes('coomer') || host.includes('cum.st') || host.includes('onlyhaven')) return 'onlyhaven';
      if (host === 'gumroad.com' || host.endsWith('.gumroad.com')) return 'gumroad';
      if (host.includes('mega.nz') || host.includes('mega.co.nz')) return 'mega';
      if (host.includes('pixeldrain.com')) return 'pixeldrain';
      if (host.includes('dropbox.com')) return 'dropbox';
      if (host.includes('drive.google.com')) return 'googledrive';
      if (['bit.ly', 'buff.ly', 'cutt.ly', 'goo.gl', 'is.gd', 'lnkd.in', 'ow.ly', 'rb.gy', 'rebrand.ly', 'shorturl.at', 't.co', 'tiny.one', 'tinyurl.com', 'v.gd', 'x.gd'].includes(host)) return 'shortlink';
      return null;
    } catch {
      return null;
    }
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

  export function deriveCloudProviderFromUrl(url: string): string {
    try {
      const u = new URL(url);
      const host = u.hostname.replace(/^www\./, '').toLowerCase();
      if (host.includes('b-cdn.net') || host.includes('bunny.net') || host.includes('mediadelivery.net')) return 'Bunny';
      if (host.includes('proton.me') || host.includes('protondrive.com')) return 'Proton';
      if (host.includes('gofile.io')) return 'Gofile';
      if (host.includes('mediafire.com')) return 'MediaFire';
      if (host.includes('terabox.com') || host.includes('1024tera.com')) return 'TeraBox';
      if (host.includes('catbox.moe')) return 'Catbox';
      if (host.includes('workupload.com')) return 'WorkUpload';
      if (host.includes('qiwi.gg')) return 'Qiwi';
      if (host.includes('send.cm')) return 'Send.cm';
      if (host.includes('mega.nz') || host.includes('mega.co.nz')) return 'MEGA';
      if (host.includes('dropbox.com')) return 'Dropbox';
      if (host.includes('pixeldrain.com')) return 'Pixeldrain';
      if (host.includes('drive.google.com')) return 'Google Drive';
      const parts = host.split('.');
      if (parts.length >= 2) {
        const name = parts[parts.length - 2];
        return name.charAt(0).toUpperCase() + name.slice(1);
      }
      return host;
    } catch {
      return 'Cloud';
    }
  }

  export function extractCloudLinks(raw: string): string[] {
    if (!raw) return [];
    const regex = /https?:\/\/(?:[a-zA-Z0-9-]+\.)*(?:mega\.nz|mega\.co\.nz|pixeldrain\.com|dropbox\.com|drive\.google\.com|iframely\.net|iframe\.ly)\/[^\s<>"')]+/gi;
    const matches = raw.match(regex) || [];
    return [...new Set(matches)];
  }

  function safeHttpUrl(raw: string): string | null {
    try {
      const url = new URL(raw);
      return ['http:', 'https:'].includes(url.protocol) ? url.href : null;
    } catch {
      return null;
    }
  }

  export function preprocessRichContent(content: string): string {
    if (!content) return '';
    let res = content;

    // Unescape encoded HTML tags like &lt;strong&gt;, &lt;em&gt;, &lt;p&gt;, &lt;br&gt;, &lt;a href=...&gt;
    if (res.includes('&lt;') && res.includes('&gt;')) {
      res = res.replace(/&lt;(\/?(?:strong|b|em|i|u|s|del|p|br|div|span|h[1-6]|ul|ol|li|blockquote|a|code|pre)(?:\s+[^&>]*)?)&gt;/gi, '<$1>');
    }

    // Convert markdown links: [text](https://...) -> <a href="https://...">text</a>
    res = res.replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2">$1</a>');

    // Convert markdown bold: **bold** or __bold__ -> <strong>bold</strong>
    res = res.replace(/(\*{2}|_{2})(.*?)\1/g, '<strong>$2</strong>');

    // Convert markdown italic: *italic* or _italic_ -> <em>italic</em>
    res = res.replace(/(^|[^\w*])\*([^\*\n]+)\*([^\w*]|$)/g, '$1<em>$2</em>$3');
    res = res.replace(/(^|[^\w_])_([^\_\n]+)_([^\w_]|$)/g, '$1<em>$2</em>$3');

    // Convert markdown strikethrough: ~~strike~~ -> <del>strike</del>
    res = res.replace(/~~(.*?)~~/g, '<del>$1</del>');

    // Convert markdown code: `code` -> <code>code</code>
    res = res.replace(/`([^`]+)`/g, '<code>$1</code>');

    // If there are no HTML paragraph/break tags, preserve newlines as <br>
    if (!/<(p|br|div|h[1-6]|ul|ol|li|blockquote)[^>]*>/i.test(res)) {
      res = res.replace(/\n/g, '<br>');
    }

    return res;
  }

  export function sanitizeRichHtml(html: string): string {
    if (typeof DOMParser === 'undefined' || !html) return '';
    const preprocessed = preprocessRichContent(html);
    const document = new DOMParser().parseFromString(preprocessed, 'text/html');
    for (const element of [...document.body.querySelectorAll('*')]) {
      const tag = element.tagName.toLocaleLowerCase();
      if (DROP_TAGS.has(tag)) {
        element.remove();
        continue;
      }
      if (!ALLOWED_TAGS.has(tag)) {
        element.replaceWith(...element.childNodes);
        continue;
      }

      const href = element instanceof HTMLAnchorElement
        ? safeHttpUrl(element.getAttribute('href') || '')
        : null;
      const title = element.getAttribute('title')?.slice(0, 500);
      const imageSource = element instanceof HTMLImageElement
        ? safeHttpUrl(element.getAttribute('src') || '')
        : null;
      const imageAlt = element instanceof HTMLImageElement
        ? (element.getAttribute('alt') || '').slice(0, 500)
        : '';
      const iframeSource = element instanceof HTMLIFrameElement
        ? safeHttpUrl(element.getAttribute('src') || '')
        : null;

      for (const attribute of [...element.attributes]) element.removeAttribute(attribute.name);
      if (element instanceof HTMLAnchorElement) {
        if (href) {
          element.href = href;
          element.rel = 'noopener noreferrer nofollow';
          if (title) element.title = title;
        }
      } else if (element instanceof HTMLImageElement) {
        if (imageSource) element.src = imageSource;
        else {
          element.remove();
          continue;
        }
        element.alt = imageAlt;
        element.loading = 'lazy';
        element.decoding = 'async';
        element.referrerPolicy = 'no-referrer';
      } else if (element instanceof HTMLIFrameElement) {
        if (iframeSource && isSafeIframeSrc(iframeSource)) {
          element.src = iframeSource;
          element.loading = 'lazy';
          element.referrerPolicy = 'no-referrer';
          element.setAttribute('sandbox', 'allow-scripts allow-same-origin allow-forms allow-popups allow-presentation');
          element.setAttribute('allowfullscreen', 'true');
          element.classList.add('rich-content-iframe');
        } else {
          element.remove();
          continue;
        }
      }
    }
    return document.body.innerHTML;
  }

  export function resolveSmartLink(
    href: string,
    currentService?: string,
    currentCreatorId?: string
  ): Promise<ResolvedPostLink | null> {
    const key = `${currentService || ''}:${currentCreatorId || ''}:${href}`;
    let pending = resolutionCache.get(key);
    if (!pending) {
      pending = apiResolveExternalPostLink(href, currentService, currentCreatorId).then(
        (resolved) => {
          if (!resolved) resolutionCache.delete(key);
          return resolved;
        },
        () => {
          resolutionCache.delete(key);
          return null;
        }
      );
      resolutionCache.set(key, pending);
    }
    return pending;
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import { i18n } from '$lib/i18n';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { apiOpenInBrowser } from '$lib/utils/ipc';
  import { toast } from 'svelte-sonner';
  import { ripple } from '$lib/motion';
  import IconFolder from '~icons/fluent/folder-open-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconCopy from '~icons/fluent/copy-24-regular';

  interface Props {
    html: string;
    currentService?: string;
    currentCreatorId?: string;
    onopencloud?: (url: string) => void;
  }

  interface LinkPopoverState {
    url: string;
    x: number;
    y: number;
    canOpenInApp: boolean;
    isCloud?: boolean;
    resolvedPost?: ResolvedPostLink;
  }

  let { html, currentService, currentCreatorId, onopencloud }: Props = $props();
  let root = $state<HTMLDivElement>();
  let generation = 0;
  let safeHtml = $derived(sanitizeRichHtml(html));
  let linkPopover = $state<LinkPopoverState | null>(null);

  function markResolved(anchor: HTMLAnchorElement, resolved: ResolvedPostLink | null) {
    if (!root?.contains(anchor)) return;
    if (resolved) anchor.dataset.linkPlatform = resolved.platform;
    anchor.dataset.smartState = resolved ? 'resolved' : 'external';
    anchor.title = i18n.t(resolved ? 'post.link_open_internal' : 'post.link_open_external');
  }

  const POST_PLATFORMS = new Set([
    'patreon', 'fanbox', 'fantia', 'boosty', 'subscribestar',
    'afdian', 'candfans', 'onlyfans', 'fansly', 'pawchive', 'onlyhaven'
  ]);

  const CLOUD_PLATFORMS = new Set(['mega', 'dropbox', 'pixeldrain', 'googledrive']);

  async function enhanceLinks() {
    await tick();
    if (!root) return;
    const currentGeneration = ++generation;
    const anchors = [...root.querySelectorAll<HTMLAnchorElement>('a[href]')];
    const smartAnchors: HTMLAnchorElement[] = [];
    for (const anchor of anchors) {
      const platform = smartLinkPlatform(anchor.href);
      anchor.dataset.linkPlatform = platform || 'external';
      anchor.title ||= i18n.t('post.link_open_external');
      if (platform && (POST_PLATFORMS.has(platform) || platform === 'shortlink')) {
        anchor.dataset.smartState = 'checking';
        smartAnchors.push(anchor);
      }
    }

    let cursor = 0;
    const worker = async () => {
      while (cursor < smartAnchors.length && currentGeneration === generation) {
        const anchor = smartAnchors[cursor++];
        const resolved = await resolveSmartLink(anchor.href, currentService, currentCreatorId);
        if (currentGeneration === generation) markResolved(anchor, resolved);
      }
    };
    await Promise.all(Array.from({ length: Math.min(3, smartAnchors.length) }, worker));
  }

  async function handleClick(event: MouseEvent) {
    if (!(event.target instanceof Element)) return;
    const target = event.target;
    const anchor = target.closest<HTMLAnchorElement>('a[href]');
    if (!anchor || !root?.contains(anchor)) return;
    event.preventDefault();
    event.stopPropagation();

    const url = anchor.href;
    const platform = smartLinkPlatform(url);

    // 1. Cloud folder links (MEGA, Dropbox, Pixeldrain, Google Drive)
    if (platform && CLOUD_PLATFORMS.has(platform)) {
      linkPopover = {
        url,
        x: event.clientX,
        y: event.clientY,
        canOpenInApp: Boolean(onopencloud),
        isCloud: true
      };
      return;
    }

    // 2. Creator Post Smart links (Patreon, Fanbox, Fantia, Boosty, etc.)
    if (platform && (POST_PLATFORMS.has(platform) || platform === 'shortlink')) {
      anchor.dataset.smartState = 'checking';
      const resolved = await resolveSmartLink(url, currentService, currentCreatorId);
      markResolved(anchor, resolved);
      if (resolved) {
        linkPopover = {
          url,
          x: event.clientX,
          y: event.clientY,
          canOpenInApp: true,
          resolvedPost: resolved
        };
        return;
      }
    }

    // 3. Regular external links (Proton Drive, Bunny, Gofile, MediaFire, general web links)
    // Directly open external URL in the default browser so it works instantly!
    void apiOpenInBrowser(url);
  }

  $effect(() => {
    safeHtml;
    currentService;
    currentCreatorId;
    void enhanceLinks();
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div bind:this={root} class="rich-content-root" onclick={handleClick}>{@html safeHtml}</div>

{#if linkPopover}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 bg-transparent"
    onclick={() => (linkPopover = null)}
  >
    <div
      class="link-popover floating-surface absolute z-50 min-w-[200px] flex flex-col gap-[var(--floating-gap)] text-xs"
      style="left: {Math.max(12, Math.min(linkPopover.x, (typeof window !== 'undefined' ? window.innerWidth : 800) - 220))}px; top: {Math.max(12, Math.min(linkPopover.y + 10, (typeof window !== 'undefined' ? window.innerHeight : 600) - 150))}px;"
      onclick={(e) => e.stopPropagation()}
    >
      {#if linkPopover.canOpenInApp}
        <button
          type="button"
          class="floating-item font-medium"
          use:ripple
          onclick={() => {
            const state = linkPopover!;
            linkPopover = null;
            if (state.isCloud && onopencloud) {
              onopencloud(state.url);
            } else if (state.resolvedPost) {
              if (state.resolvedPost.link_type === 'creator' || !state.resolvedPost.post_id) {
                navigationState.openCreator(state.resolvedPost.service, state.resolvedPost.creator_id);
              } else {
                navigationState.openPost(
                  state.resolvedPost.service,
                  state.resolvedPost.creator_id,
                  state.resolvedPost.post_id
                );
              }
            } else {
              void apiOpenInBrowser(state.url);
            }
          }}
        >
          <IconFolder class="w-5 h-5 text-[var(--accent)] flex-shrink-0" />
          <span class="text-[var(--text-primary)]">
            {linkPopover.resolvedPost?.link_type === 'creator'
              ? (i18n.t('post.open_creator_in_app') || i18n.t('post.open_in_app') || 'Open in App')
              : (i18n.t('post.open_in_app') || 'Open in App')}
          </span>
        </button>
      {/if}

      <button
        type="button"
        class="floating-item"
        use:ripple
        onclick={() => {
          const u = linkPopover!.url;
          linkPopover = null;
          void apiOpenInBrowser(u);
        }}
      >
        <IconOpen class="w-5 h-5 text-[var(--text-muted)] flex-shrink-0" />
        <span>{i18n.t('post.open_in_browser') || 'Open in Browser'}</span>
      </button>

      <button
        type="button"
        class="floating-item"
        use:ripple
        onclick={() => {
          const u = linkPopover!.url;
          linkPopover = null;
          navigator.clipboard.writeText(u);
          toast.success(i18n.t('post.link_copied') || 'Link copied');
        }}
      >
        <IconCopy class="w-5 h-5 text-[var(--text-muted)] flex-shrink-0" />
        <span>{i18n.t('post.copy_link') || 'Copy Link'}</span>
      </button>
    </div>
  </div>
{/if}

<style>
  .rich-content-root { display: contents; }

  .rich-content-root :global(a[href]) {
    --smart-link-color: var(--accent-primary);
    display: inline;
    overflow-wrap: anywhere;
  }

  .rich-content-root :global(a[href])::before {
    content: '';
    display: inline-block;
    width: max(1.08em, 16px);
    height: max(1.08em, 16px);
    margin-right: 0.38em;
    background: currentColor;
    vertical-align: -0.18em;
    mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath fill='black' d='M9.25 7a.75.75 0 0 1 .11 1.492l-.11.008H7a3.5 3.5 0 0 0-.206 6.994L7 15.5h2.25a.75.75 0 0 1 .11 1.492L9.25 17H7a5 5 0 0 1-.25-9.994L7 7zM17 7a5 5 0 0 1 .25 9.994L17 17h-2.25a.75.75 0 0 1-.11-1.492l.11-.008H17a3.5 3.5 0 0 0 .206-6.994L17 8.5h-2.25a.75.75 0 0 1-.11-1.492L14.75 7zM7 11.25h10a.75.75 0 0 1 .102 1.493L17 12.75H7a.75.75 0 0 1-.102-1.493zh10z'/%3E%3C/svg%3E") center / contain no-repeat;
  }

  .rich-content-root :global(a[data-link-platform='patreon'])::before {
    mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath fill='black' d='M22.957 7.21c-.004-3.064-2.391-5.576-5.191-6.482-3.478-1.125-8.064-.962-11.384.604C2.357 3.231 1.093 7.391 1.046 11.54c-.039 3.411.302 12.396 5.369 12.46 3.765.047 4.326-4.804 6.068-7.141 1.24-1.662 2.836-2.132 4.801-2.618 3.376-.836 5.678-3.501 5.673-7.031'/%3E%3C/svg%3E");
  }

  .rich-content-root :global(a[data-link-platform='fanbox'])::before {
    mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath fill='black' d='M4.94 0A4.953 4.953 0 0 0 0 4.94v14.12A4.953 4.953 0 0 0 4.94 24h14.12A4.953 4.953 0 0 0 24 19.06V4.94A4.953 4.953 0 0 0 19.06 0Zm1.783 5.465h.904a.37.37 0 0 1 .31.17l.752 1.17a6.172 6.172 0 0 1 10.01 4.834 6.172 6.172 0 0 1-9.394 5.265v2.016a.37.37 0 0 1-.37.367H6.724a.37.37 0 0 1-.37-.367V5.834a.37.37 0 0 1 .37-.37m5.804 2.951a3.222 3.222 0 1 0-.002 6.443 3.222 3.222 0 0 0 .002-6.443'/%3E%3C/svg%3E");
  }

  .rich-content-root :global(a[data-link-platform='boosty'])::before {
    mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath fill='black' d='M2.661 14.337 6.801 0h6.362L11.88 4.444 8.464 16.254h3.15q-1.982 4.934-3.086 7.733c-5.816-.063-7.442-4.228-6.02-9.155M8.554 24l7.67-11.035h-3.25l2.83-7.073c4.852.508 7.137 4.33 5.791 8.952C20.16 19.81 14.344 24 8.68 24z'/%3E%3C/svg%3E");
  }

  .rich-content-root :global(a[data-link-platform='patreon']) { --smart-link-color: #ff5c5c; }
  .rich-content-root :global(a[data-link-platform='fanbox']) { --smart-link-color: #5b9dff; }
  .rich-content-root :global(a[data-link-platform='fantia']) { --smart-link-color: #ff6fae; }
  .rich-content-root :global(a[data-link-platform='subscribestar']) { --smart-link-color: #59c879; }
  .rich-content-root :global(a[data-link-platform='boosty']) { --smart-link-color: #f68b3c; }
  .rich-content-root :global(a[data-link-platform='afdian']) { --smart-link-color: #9b7cff; }
  .rich-content-root :global(a[data-link-platform='gumroad']) { --smart-link-color: #ff90e8; }
  .rich-content-root :global(a[data-link-platform='shortlink']) { --smart-link-color: #a78bfa; }
  .rich-content-root :global(a[data-link-platform='proton']) { --smart-link-color: #7b57ff; }
  .rich-content-root :global(a[data-link-platform='bunny']) { --smart-link-color: #ff8300; }
  .rich-content-root :global(a[data-link-platform='gofile']) { --smart-link-color: #3b82f6; }
  .rich-content-root :global(a[data-link-platform='mediafire']) { --smart-link-color: #0070f3; }
  .rich-content-root :global(a[data-link-platform='terabox']) { --smart-link-color: #06b6d4; }
  .rich-content-root :global(a[data-link-platform='catbox']) { --smart-link-color: #f43f5e; }
  .rich-content-root :global(a[data-link-platform='workupload']) { --smart-link-color: #10b981; }
  .rich-content-root :global(a[data-link-platform='qiwi']) { --smart-link-color: #f59e0b; }
  .rich-content-root :global(a[data-link-platform='sendcm']) { --smart-link-color: #ec4899; }
  .rich-content-root :global(a[data-link-platform='mega']) { --smart-link-color: #ef4444; }
  .rich-content-root :global(a[data-link-platform='dropbox']) { --smart-link-color: #3b82f6; }
  .rich-content-root :global(a[data-link-platform='pixeldrain']) { --smart-link-color: #a855f7; }
  .rich-content-root :global(a[data-link-platform='googledrive']) { --smart-link-color: #10b981; }

  .rich-content-root :global(a[data-link-platform]:not([data-link-platform='external'])) {
    color: var(--smart-link-color);
    text-decoration-color: color-mix(in srgb, var(--smart-link-color) 50%, transparent);
  }

  .rich-content-root :global(a[data-smart-state='resolved']) {
    padding: 0.08em 0.28em;
    border-radius: 0.35em;
    background: color-mix(in srgb, var(--smart-link-color) 13%, transparent);
    text-decoration-color: transparent;
  }

  .rich-content-root :global(a[data-smart-state='checking'])::before {
    animation: smart-link-pulse 800ms ease-in-out infinite alternate;
  }

  @keyframes smart-link-pulse {
    to { opacity: 0.32; }
  }

  .rich-content-root :global(iframe.rich-content-iframe) {
    width: 100%;
    min-height: 520px;
    height: 700px;
    max-height: 85vh;
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.12));
    border-radius: 12px;
    background: var(--bg-card, #1c1c1f);
    margin: 1rem 0;
    display: block;
  }

  .rich-content-root :global(.iframely-responsive) {
    position: relative;
    width: 100%;
  }

  @media (prefers-reduced-motion: reduce) {
    .rich-content-root :global(a[data-smart-state='checking'])::before { animation: none; }
  }
</style>
