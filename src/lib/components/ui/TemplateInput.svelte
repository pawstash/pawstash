<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { computePosition, autoUpdate, flip, shift, offset, size } from '@floating-ui/dom';
  import { portal } from '$lib/actions/portal';
  import { scrollable } from '$lib/actions/scrollable';
  import { i18n } from '$lib/i18n';
  import { ripple, tooltip } from '$lib/motion';
  import { notify } from '$lib/utils/toast';
  import IconCode from '~icons/fluent/code-24-regular';
  import IconEye from '~icons/fluent/eye-24-regular';

  export interface TemplateTag {
    tag: string;
    label: string;
    example?: string;
  }

  interface Props {
    value?: string;
    placeholder?: string;
    tags?: TemplateTag[];
    previewType?: 'creator' | 'post' | 'file';
    disabled?: boolean;
    onchange?: (value: string) => void;
    onblur?: () => void;
    class?: string;
  }

  let {
    value = $bindable(''),
    placeholder = '',
    tags = [],
    previewType = 'file',
    disabled = false,
    onchange,
    onblur,
    class: extraClass = ''
  }: Props = $props();

  let inputEl = $state<HTMLInputElement>();
  let rootEl = $state<HTMLDivElement>();
  let dropdownEl = $state<HTMLDivElement | null>(null);
  let isOpen = $state(false);
  let selectedIndex = $state(0);
  let cleanupAutoUpdate: (() => void) | null = null;
  let closeTimeout: ReturnType<typeof setTimeout> | undefined;

  let filterQuery = $state('');

  let filteredTags = $derived.by(() => {
    if (!filterQuery.trim()) return tags;
    const q = filterQuery.toLowerCase().replace(/[{}]/g, '');
    return tags.filter(
      (t) =>
        t.tag.toLowerCase().includes(q) ||
        t.label.toLowerCase().includes(q) ||
        (t.example && t.example.toLowerCase().includes(q))
    );
  });

  function getPreview(template: string, type: 'creator' | 'post' | 'file'): string {
    let t = (template || '').trim();
    if (type === 'creator') {
      if (!t) t = '{creator}';
      return t
        .replaceAll('{creator}', 'AuthorName')
        .replaceAll('{author}', 'AuthorName')
        .replaceAll('{name}', 'AuthorName')
        .replaceAll('{service}', 'Platform')
        .replaceAll('{platform}', 'Platform')
        .replaceAll('{creator_id}', '12345')
        .replaceAll('{id}', '12345')
        .replaceAll('{date}', '2024-08-20')
        .replaceAll('{published}', '2024-08-20')
        .replaceAll('{date_compact}', '20240820')
        .replaceAll('{date_dots}', '2024.08.20')
        .replaceAll('{year}', '2024')
        .replaceAll('{yyyy}', '2024')
        .replaceAll('{year_short}', '24')
        .replaceAll('{yy}', '24')
        .replaceAll('{month}', '08')
        .replaceAll('{mm}', '08')
        .replaceAll('{day}', '20')
        .replaceAll('{dd}', '20')
        .replaceAll('{year_month}', '2024-08');
    }
    if (type === 'post') {
      if (!t) t = '{post_title}';
      return t
        .replaceAll('{post_title}', 'PostTitle')
        .replaceAll('{title}', 'PostTitle')
        .replaceAll('{post_id}', '67890')
        .replaceAll('{id}', '67890')
        .replaceAll('{creator}', 'AuthorName')
        .replaceAll('{author}', 'AuthorName')
        .replaceAll('{name}', 'AuthorName')
        .replaceAll('{service}', 'Platform')
        .replaceAll('{platform}', 'Platform')
        .replaceAll('{date}', '2024-08-20')
        .replaceAll('{published}', '2024-08-20')
        .replaceAll('{date_compact}', '20240820')
        .replaceAll('{date_dots}', '2024.08.20')
        .replaceAll('{year}', '2024')
        .replaceAll('{yyyy}', '2024')
        .replaceAll('{year_short}', '24')
        .replaceAll('{yy}', '24')
        .replaceAll('{month}', '08')
        .replaceAll('{mm}', '08')
        .replaceAll('{day}', '20')
        .replaceAll('{dd}', '20')
        .replaceAll('{year_month}', '2024-08');
    }
    if (!t) t = '{post_title} - {filename}';
    let res = t
      .replaceAll('{post_title}', 'PostTitle')
      .replaceAll('{title}', 'PostTitle')
      .replaceAll('{post_id}', '67890')
      .replaceAll('{creator}', 'AuthorName')
      .replaceAll('{author}', 'AuthorName')
      .replaceAll('{service}', 'Platform')
      .replaceAll('{platform}', 'Platform')
      .replaceAll('{filename}', 'OriginalFilename.png')
      .replaceAll('{original_name}', 'OriginalFilename.png')
      .replaceAll('{name}', 'OriginalFilename')
      .replaceAll('{ext}', 'png')
      .replaceAll('{index}', '1')
      .replaceAll('{date}', '2024-08-20')
      .replaceAll('{published}', '2024-08-20')
      .replaceAll('{date_compact}', '20240820')
      .replaceAll('{date_dots}', '2024.08.20')
      .replaceAll('{year}', '2024')
      .replaceAll('{yyyy}', '2024')
      .replaceAll('{year_short}', '24')
      .replaceAll('{yy}', '24')
      .replaceAll('{month}', '08')
      .replaceAll('{mm}', '08')
      .replaceAll('{day}', '20')
      .replaceAll('{dd}', '20')
      .replaceAll('{year_month}', '2024-08')
      .replaceAll('{media_id}', 'MediaID');

    if (res.toLowerCase().endsWith('.png')) {
      return res;
    }
    const cleanStem = res.replace(/\.png/gi, '');
    return `${cleanStem}.png`;
  }

  let previewValue = $derived(getPreview(value, previewType));

  async function updatePosition() {
    if (!rootEl || !dropdownEl) return;
    const { x, y } = await computePosition(rootEl, dropdownEl, {
      placement: 'bottom-start',
      strategy: 'fixed',
      middleware: [
        offset(6),
        flip({ fallbackPlacements: ['top-start', 'bottom-end', 'top-end'], padding: 12 }),
        shift({ padding: 12 }),
        size({
          padding: 12,
          apply({ availableHeight, elements }) {
            Object.assign(elements.floating.style, {
              maxHeight: `${Math.min(availableHeight, 320)}px`
            });
          }
        })
      ]
    });

    if (!dropdownEl) return;
    dropdownEl.style.left = `${x}px`;
    dropdownEl.style.top = `${y}px`;
    dropdownEl.style.visibility = 'visible';
  }

  $effect(() => {
    if (isOpen && rootEl && dropdownEl) {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = autoUpdate(rootEl, dropdownEl, () => {
        void updatePosition();
      });
    } else {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = null;
    }
    return () => {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = null;
    };
  });

  function openDropdown() {
    if (closeTimeout) clearTimeout(closeTimeout);
    filterQuery = '';
    selectedIndex = 0;
    isOpen = true;
  }

  function scheduleClose() {
    closeTimeout = setTimeout(() => {
      isOpen = false;
      onblur?.();
    }, 180);
  }

  function handleFocus() {
    openDropdown();
  }

  function handleInput(e: Event) {
    const target = e.currentTarget as HTMLInputElement;
    value = target.value;
    onchange?.(value);

    const cursor = target.selectionStart ?? target.value.length;
    const beforeCursor = target.value.slice(0, cursor);
    const lastOpenBrace = beforeCursor.lastIndexOf('{');
    const lastCloseBrace = beforeCursor.lastIndexOf('}');

    if (lastOpenBrace > lastCloseBrace) {
      filterQuery = beforeCursor.slice(lastOpenBrace);
      isOpen = true;
    } else {
      filterQuery = '';
    }
  }

  function insertTag(tag: string) {
    if (!inputEl) {
      value = value ? `${value}${tag}` : tag;
      onchange?.(value);
      isOpen = false;
      return;
    }

    const start = inputEl.selectionStart ?? value.length;
    const end = inputEl.selectionEnd ?? value.length;

    const before = value.slice(0, start);
    const after = value.slice(end);
    const lastOpen = before.lastIndexOf('{');
    const lastClose = before.lastIndexOf('}');

    let newBefore = before;
    if (lastOpen > lastClose && lastOpen >= 0) {
      newBefore = before.slice(0, lastOpen);
    }

    value = `${newBefore}${tag}${after}`;
    onchange?.(value);

    const newCursor = newBefore.length + tag.length;
    isOpen = false;

    void tick().then(() => {
      inputEl?.focus();
      inputEl?.setSelectionRange(newCursor, newCursor);
    });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!isOpen) {
      if (e.key === 'ArrowDown' || e.key === 'Enter') {
        openDropdown();
        e.preventDefault();
      }
      return;
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % Math.max(1, filteredTags.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + filteredTags.length) % Math.max(1, filteredTags.length);
    } else if (e.key === 'Enter' || e.key === 'Tab') {
      if (filteredTags.length > 0 && selectedIndex < filteredTags.length) {
        e.preventDefault();
        insertTag(filteredTags[selectedIndex].tag);
      }
    } else if (e.key === 'Escape') {
      isOpen = false;
    }
  }
</script>

<div class="template-input-wrapper {extraClass}" bind:this={rootEl}>
  <!-- svelte-ignore css_unused_selector -->
  <div style="position:relative; width:100%;">
    <input
      bind:this={inputEl}
      type="text"
      {placeholder}
      {disabled}
      bind:value
      onfocus={handleFocus}
      onblur={scheduleClose}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      class="template-input-field"
      style="padding-right: 74px;"
      autocomplete="off"
      spellcheck="false"
    />

    <div style="position:absolute; right:12px; top:50%; transform:translateY(-50%); display:flex; align-items:center; gap:4px; z-index:10;">
      {#if previewValue}
        <button
          type="button"
          use:ripple
          use:tooltip={previewValue}
          onclick={(e) => {
            e.stopPropagation();
            notify.info(previewValue);
          }}
          style="width:22px; height:22px; border:none; outline:none; background:transparent; display:flex; align-items:center; justify-content:center; color:white; opacity:0.45; cursor:pointer; padding:0; transition:opacity 200ms ease, color 200ms ease;"
          onmouseenter={(e) => { e.currentTarget.style.opacity = '0.9'; }}
          onmouseleave={(e) => { e.currentTarget.style.opacity = '0.45'; }}
          aria-label="Preview"
          tabindex="-1"
        >
          <IconEye style="width: 19px; height: 19px;" />
        </button>
      {/if}

      <button
        type="button"
        use:ripple
        onclick={(e) => {
          e.stopPropagation();
          if (isOpen) isOpen = false;
          else {
            inputEl?.focus();
            openDropdown();
          }
        }}
        style="width:22px; height:22px; border:none; outline:none; background:transparent; display:flex; align-items:center; justify-content:center; color:white; opacity:{isOpen ? '0.9' : '0.45'}; cursor:pointer; padding:0; transition:opacity 200ms ease, color 200ms ease;"
        onmouseenter={(e) => { e.currentTarget.style.opacity = '0.9'; }}
        onmouseleave={(e) => { if (!isOpen) e.currentTarget.style.opacity = '0.45'; }}
        title="Insert variable"
        aria-label="Insert variable"
        tabindex="-1"
      >
        <IconCode style="width: 19px; height: 19px;" />
      </button>
    </div>
  </div>
</div>

{#if isOpen && filteredTags.length > 0}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    use:portal
    bind:this={dropdownEl}
    class="template-dropdown"
    role="dialog"
    aria-label="Variables"
    tabindex="-1"
    onmousedown={(e) => e.preventDefault()}
  >
    <div class="dropdown-list" use:scrollable>
      {#each filteredTags as item, index (item.tag)}
        <button
          type="button"
          class="dropdown-item"
          class:selected={index === selectedIndex}
          onclick={() => insertTag(item.tag)}
          onmouseenter={() => (selectedIndex = index)}
        >
          <span class="tag-label">{item.label}</span>
          <code class="tag-badge">{item.tag}</code>
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .template-input-wrapper {
    display: flex;
    flex-direction: column;
    width: 100%;
    position: relative;
  }

  .template-input-field {
    display: block;
    width: 100%;
    height: 46px;
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 14px;
    padding: 0 74px 0 14px;
    outline: none;
    box-sizing: border-box;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo),
                box-shadow var(--duration-fast) var(--ease-expo);
  }

  .template-input-field::placeholder {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .template-input-field:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
  }

  .template-input-field:focus {
    background: var(--bg-card-hover);
    border-color: var(--border-color-focus);
    box-shadow: 0 0 0 2px var(--accent-glow);
  }

  .template-dropdown {
    position: fixed;
    z-index: 10000;
    width: min(320px, calc(100vw - 32px));
    background: var(--bg-surface-raised, #16161a);
    backdrop-filter: blur(28px);
    -webkit-backdrop-filter: blur(28px);
    border: 1px solid var(--border-color, rgba(255, 255, 255, 0.12));
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.05);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    visibility: hidden;
    padding: 4px;
    animation: dropdownFadeIn 140ms var(--ease-expo);
  }

  @keyframes dropdownFadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .dropdown-list {
    display: flex;
    flex-direction: column;
    padding: 2px;
    max-height: 240px;
    width: 100%;
    box-sizing: border-box;
  }

  .dropdown-list :global(.os-viewport) {
    width: 100%;
    box-sizing: border-box;
  }

  .dropdown-list :global(.os-content) {
    display: flex !important;
    flex-direction: column !important;
    width: 100% !important;
    box-sizing: border-box !important;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-md, 8px);
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: background var(--duration-fast), color var(--duration-fast);
    box-sizing: border-box;
    flex-shrink: 0;
  }

  .dropdown-item:hover,
  .dropdown-item.selected {
    background: rgba(255, 255, 255, 0.08);
  }

  .dropdown-item.selected {
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .tag-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary, #ffffff);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tag-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: rgba(255, 255, 255, 0.06);
    padding: 2px 6px;
    border-radius: 4px;
    flex-shrink: 0;
  }
</style>
