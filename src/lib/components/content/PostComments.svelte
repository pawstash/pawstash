<script lang="ts">
  import { getContext, untrack } from 'svelte';
  import type { Comment } from '$lib/types/content';
  import type { CommentCloudLinks } from '$lib/types/cloud';
  import { i18n } from '$lib/i18n';
  import { logger } from '$lib/utils/logger';
  import { apiFetchPostComments } from '$lib/utils/ipc';
  import { formatDate } from '$lib/utils/formatters';
  import { sortOption } from '$lib/utils/sort';
  import { extractCloudLinks } from '$lib/utils/media';
  import { collapseSection } from '$lib/utils/collapseSection';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import Button from '$lib/components/ui/Button.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import SortControl from '$lib/components/layout/SortControl.svelte';
  import RichContent from './RichContent.svelte';
  import IconComment from '~icons/fluent/comment-24-regular';
  import IconChevronDown from '~icons/fluent/chevron-down-24-regular';
  import IconChevronUp from '~icons/fluent/chevron-up-24-regular';

  interface Props {
    service: string;
    creatorId: string;
    postId: string;
    onopencloud?: (url: string, text?: string) => void;
    onopenmedia?: (src: string, alt?: string) => void;
    oncloudlinks?: (links: CommentCloudLinks) => void;
  }

  let { service, creatorId, postId, onopencloud, onopenmedia, oncloudlinks }: Props = $props();

  const MAX_HEIGHT = 600;
  const SEARCH_MIN = 20;
  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);

  let comments = $state<Comment[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let expanded = $state(false);
  let listHeight = $state(0);
  let sort = $state<'newest' | 'oldest'>('newest');
  let searchQuery = $state('');
  let searchOpen = $state(false);
  let wrapperEl = $state<HTMLElement | null>(null);
  let scrollBeforeExpand = $state<number | null>(null);

  $effect(() => {
    const all = new Set<string>();
    const author = new Set<string>();
    const others = new Set<string>();
    for (const comment of comments) {
      for (const url of extractCloudLinks(comment.content)) {
        all.add(url);
        if (comment.commenter === creatorId) author.add(url);
        else others.add(url);
      }
    }
    untrack(() => oncloudlinks?.({ all: [...all], author: [...author], others: [...others] }));
  });

  const sortOptions = [
    sortOption<'newest' | 'oldest'>('newest', 'published', 'desc'),
    sortOption<'newest' | 'oldest'>('oldest', 'published', 'asc')
  ];

  let query = $derived(searchQuery.trim().toLowerCase());
  let rootComments = $derived.by(() => {
    const ids = new Set(comments.map((c) => c.id));
    return comments
      .filter((c) => !c.parent_id || !ids.has(c.parent_id))
      .sort((a, b) => {
        const timeA = new Date(a.published || 0).getTime();
        const timeB = new Date(b.published || 0).getTime();
        return sort === 'newest' ? timeB - timeA : timeA - timeB;
      });
  });
  let visibleRootComments = $derived(query ? rootComments.filter(threadMatches) : rootComments);
  let collapsible = $derived(listHeight > MAX_HEIGHT && !query);

  export function reload() {
    void load();
  }

  async function load() {
    if (!postId) return;
    loading = true;
    error = null;
    try {
      comments = await apiFetchPostComments(service, creatorId, postId);
    } catch (err) {
      const message = String(err);
      if (message.includes('404') || message.toLowerCase().includes('not found')) {
        comments = [];
      } else {
        logger.error(`Failed to load comments for ${service}:${postId}`, err);
        error = message;
      }
    } finally {
      loading = false;
    }
  }

  function getReplies(commentId: string) {
    return comments
      .filter((c) => c.parent_id === commentId)
      .sort((a, b) => new Date(a.published || 0).getTime() - new Date(b.published || 0).getTime());
  }

  function threadMatches(comment: Comment): boolean {
    const author = (comment.commenter_name || comment.commenter || '').toLowerCase();
    if ((comment.content ?? '').toLowerCase().includes(query) || author.includes(query)) return true;
    return getReplies(comment.id).some(threadMatches);
  }

  $effect(() => {
    if (postId) void load();
  });
</script>

{#snippet commentNode(comment: Comment)}
  <div class="comment-item">
    <div class="comment-meta">
      <span class="comment-author" class:is-creator={comment.commenter === creatorId}>
        {comment.commenter_name || comment.commenter}
      </span>
      {#if comment.commenter === creatorId}
        <span class="creator-badge">{i18n.t('post.creator_badge')}</span>
      {/if}
      <span class="comment-date">{formatDate(comment.published)}</span>
      {#if comment.revisions && comment.revisions.length > 0}
        <span class="comment-edited" title={comment.revisions.map((r) => `${formatDate(r.added)}: ${r.content}`).join('\n')}>
          ({i18n.t('post.edited')})
        </span>
      {/if}
    </div>
    <div class="comment-body">
      <RichContent
        html={comment.content}
        currentService={service}
        currentCreatorId={creatorId}
        cloudUrls={extractCloudLinks(comment.content)}
        onopencloud={(url) => onopencloud?.(url, comment.content)}
        {onopenmedia}
      />
    </div>
    {#if getReplies(comment.id).length > 0}
      <div class="comment-replies">
        {#each getReplies(comment.id) as reply (reply.id)}
          {@render commentNode(reply)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<section class="comments-section">
  <div class="comments-header-row post-section-controls" class:search-active={searchOpen}>
    <h2 class="comments-title">
      <IconComment />
      <span>{i18n.t('post.comments')}</span>
      <CountBadge count={comments.length} variant="header" />
    </h2>

    {#if comments.length > 1}
      <div class="comments-actions">
        <HeaderActions
          bind:searchOpen
          bind:searchQuery
          showSearchButton={comments.length >= SEARCH_MIN || Boolean(searchQuery)}
          searchPlaceholder={i18n.t('post.search_comments')}
          onsearchtoggle={(open) => {
            if (!open) searchQuery = '';
          }}
        >
          <SortControl variant="standalone" options={sortOptions} value={sort} onchange={(value) => (sort = value)} />
        </HeaderActions>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="comments-empty">{i18n.t('feed.loading')}</div>
  {:else if error}
    <div class="comments-empty">{error}</div>
  {:else if comments.length === 0}
    <div class="comments-empty">{i18n.t('post.no_comments')}</div>
  {:else if visibleRootComments.length === 0}
    <div class="comments-empty">{i18n.t('post.no_matching_comments', { query: searchQuery.trim() })}</div>
  {:else}
    <div class="comments-wrapper section-scroll-anchor" bind:this={wrapperEl}>
      <div class="comments-container" class:is-collapsed={collapsible && !expanded} class:is-expanded={expanded || Boolean(query)}>
        <div class="comments-list" bind:clientHeight={listHeight}>
          {#each visibleRootComments as comment (comment.id)}
            {@render commentNode(comment)}
          {/each}
        </div>
      </div>

      {#if collapsible && !expanded}
        <div class="comments-expand-action">
          <Button
            variant="ghost"
            onclick={() => {
              scrollBeforeExpand = scrollContext?.viewport?.scrollTop ?? null;
              expanded = true;
            }}
          >
            <IconChevronDown />
            <span>{i18n.t('post.expand_comments')}</span>
          </Button>
        </div>
      {:else if collapsible && expanded}
        <div class="section-collapse-action">
          <Button
            variant="ghost"
            onclick={() => collapseSection(scrollContext?.viewport, wrapperEl, MAX_HEIGHT, scrollBeforeExpand, () => (expanded = false))}
          >
            <IconChevronUp />
            <span>{i18n.t('post.collapse_comments')}</span>
          </Button>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .comments-section {
    margin-top: 48px;
    padding-top: 32px;
    border-top: 1px solid var(--outline-variant);
  }

  .comments-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 24px;
  }

  .comments-title {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    color: var(--text-primary);
    font-size: var(--type-title-md-size);
    line-height: var(--type-title-md-line);
    font-weight: 500;
  }

  .comments-title :global(svg) {
    width: 20px;
    height: 20px;
    color: var(--text-secondary);
  }

  .comments-actions {
    display: flex;
    justify-content: flex-end;
    min-width: 0;
  }

  :global([data-layout='mobile']) .comments-header-row.search-active .comments-title {
    display: none;
  }

  :global([data-layout='mobile']) .comments-header-row.search-active .comments-actions {
    flex: 1;
  }

  .comments-wrapper {
    position: relative;
    width: 100%;
  }

  .comments-container {
    max-height: 600px;
    overflow: hidden;
    transition: max-height var(--duration-slow) var(--ease-expo);
  }

  .comments-container.is-expanded {
    max-height: none;
  }

  .comments-container.is-collapsed {
    -webkit-mask-image: linear-gradient(to bottom, black 80%, transparent 100%);
    mask-image: linear-gradient(to bottom, black 80%, transparent 100%);
  }

  .comments-expand-action {
    position: relative;
    z-index: 10;
    display: flex;
    justify-content: center;
    padding: 24px 0 12px;
  }

  .comments-expand-action :global(svg),
  .section-collapse-action :global(svg) {
    width: 16px;
    height: 16px;
  }

  .comments-list {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .comment-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .comment-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: var(--type-label-lg-size);
    line-height: var(--type-label-lg-line);
  }

  .comment-author {
    color: var(--text-primary);
    font-weight: 500;
    user-select: text;
  }

  .comment-author.is-creator {
    color: var(--accent-primary);
  }

  .creator-badge {
    padding: 0 8px;
    border-radius: var(--radius-full);
    background: var(--accent-container);
    color: var(--accent-on-container);
    font-size: var(--type-label-sm-size);
    line-height: var(--type-label-sm-line);
    font-weight: 500;
  }

  .comment-date,
  .comment-edited {
    color: var(--text-secondary);
    font-size: var(--type-label-md-size);
    line-height: var(--type-label-md-line);
    user-select: text;
  }

  .comment-edited {
    font-style: italic;
    cursor: help;
  }

  .comment-body {
    color: var(--text-primary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    word-break: break-word;
    user-select: text;
  }

  .comment-body :global(*) {
    user-select: text;
  }

  .comment-body :global(a) {
    color: var(--accent-primary);
    text-decoration: none;
  }

  .comment-body :global(a:hover) {
    text-decoration: underline;
  }

  .comment-body :global(p) {
    margin: 0 0 8px;
  }

  .comment-body :global(p:last-child) {
    margin-bottom: 0;
  }

  .comment-replies {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-top: 16px;
    padding-left: 16px;
  }

  .comment-replies::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -8px;
    width: 2px;
    border-radius: 1px;
    background: var(--outline-variant);
  }

  .comments-empty {
    padding: 40px;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    text-align: center;
  }
</style>
