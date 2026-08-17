<script module lang="ts">
  import type { ResolvedPostLink } from '$lib/types/pawchive';
  import { apiResolveExternalPostLink } from '$lib/utils/ipc';

  const ALLOWED_TAGS = new Set([
    'a', 'b', 'blockquote', 'br', 'code', 'del', 'div', 'em', 'figcaption', 'figure',
    'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'hr', 'i', 'img', 'li', 'ol', 'p', 'pre',
    's', 'span', 'strong', 'sub', 'sup', 'table', 'tbody', 'td', 'th', 'thead', 'tr',
    'u', 'ul'
  ]);
  const DROP_TAGS = new Set(['base', 'button', 'embed', 'form', 'iframe', 'input', 'link', 'math', 'meta', 'object', 'script', 'style', 'svg', 'textarea']);
  const resolutionCache = new Map<string, Promise<ResolvedPostLink | null>>();

  export function smartLinkPlatform(raw: string): string | null {
    try {
      const url = new URL(raw);
      if (!['http:', 'https:'].includes(url.protocol)) return null;
      const host = url.hostname.replace(/^www\./, '').toLocaleLowerCase();
      const path = url.pathname;
      if ((host === 'patreon.com' || host.endsWith('.patreon.com')) && /\/posts\/[^/]+/i.test(path)) return 'patreon';
      if ((host === 'fanbox.cc' || host.endsWith('.fanbox.cc')) && /\/posts\/[^/]+/i.test(path)) return 'fanbox';
      if ((host === 'fantia.jp' || host.endsWith('.fantia.jp')) && /\/posts\/[^/]+/i.test(path)) return 'fantia';
      if ((host === 'subscribestar.com' || host.endsWith('.subscribestar.com') || host === 'subscribestar.adult' || host.endsWith('.subscribestar.adult')) && /\/posts\/[^/]+/i.test(path)) return 'subscribestar';
      if ((host === 'boosty.to' || host.endsWith('.boosty.to')) && /\/posts\/[^/]+/i.test(path)) return 'boosty';
      if ((host === 'afdian.com' || host.endsWith('.afdian.com') || host === 'afdian.net' || host.endsWith('.afdian.net')) && /\/p\/[^/]+/i.test(path)) return 'afdian';
      if (host === 'gumroad.com' || host.endsWith('.gumroad.com')) return 'gumroad';
      if (['bit.ly', 'buff.ly', 'cutt.ly', 'goo.gl', 'is.gd', 'lnkd.in', 'ow.ly', 'rb.gy', 'rebrand.ly', 'shorturl.at', 't.co', 'tiny.one', 'tinyurl.com', 'v.gd', 'x.gd'].includes(host)) return 'shortlink';
      return null;
    } catch {
      return null;
    }
  }

  function safeHttpUrl(raw: string): string | null {
    try {
      const url = new URL(raw);
      return ['http:', 'https:'].includes(url.protocol) ? url.href : null;
    } catch {
      return null;
    }
  }

  export function sanitizeRichHtml(html: string): string {
    if (typeof DOMParser === 'undefined' || !html) return '';
    const document = new DOMParser().parseFromString(html, 'text/html');
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

  interface Props {
    html: string;
    currentService?: string;
    currentCreatorId?: string;
  }

  let { html, currentService, currentCreatorId }: Props = $props();
  let root = $state<HTMLDivElement>();
  let generation = 0;
  let safeHtml = $derived(sanitizeRichHtml(html));

  function markResolved(anchor: HTMLAnchorElement, resolved: ResolvedPostLink | null) {
    if (!root?.contains(anchor)) return;
    if (resolved) anchor.dataset.linkPlatform = resolved.platform;
    anchor.dataset.smartState = resolved ? 'resolved' : 'external';
    anchor.title = i18n.t(resolved ? 'post.link_open_internal' : 'post.link_open_external');
  }

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
      if (platform && platform !== 'gumroad') {
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
    const platform = smartLinkPlatform(anchor.href);
    if (platform && platform !== 'gumroad') {
      anchor.dataset.smartState = 'checking';
      const resolved = await resolveSmartLink(anchor.href, currentService, currentCreatorId);
      markResolved(anchor, resolved);
      if (resolved) {
        navigationState.openPost(resolved.service, resolved.creator_id, resolved.post_id);
        return;
      }
    }
    void apiOpenInBrowser(anchor.href);
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

  @media (prefers-reduced-motion: reduce) {
    .rich-content-root :global(a[data-smart-state='checking'])::before { animation: none; }
  }
</style>
