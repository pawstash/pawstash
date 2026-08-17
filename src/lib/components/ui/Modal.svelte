<script lang="ts">
  import type { Snippet } from 'svelte';
  import IconClose from '~icons/fluent/dismiss-24-regular';

  interface Props {
    isOpen?: boolean;
    title?: string;
    size?: 'sm' | 'md' | 'lg' | 'xl';
    onclose: () => void;
    children?: Snippet;
  }

  let {
    isOpen = false,
    title = '',
    size = 'md',
    onclose,
    children
  }: Props = $props();

  const maxWidthStyles: Record<string, string> = {
    sm: 'max-width: 380px;',
    md: 'max-width: 460px;',
    lg: 'max-width: 560px;',
    xl: 'max-width: 680px;'
  };
</script>

{#if isOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4">
    <div
      role="button"
      tabindex="0"
      class="fixed inset-0 bg-black/75 backdrop-blur-md transition-opacity duration-300"
      onclick={onclose}
      onkeydown={(e) => (e.key === 'Escape' || e.key === 'Enter') && onclose()}
    ></div>

    <div
      class="modal-box relative z-10 w-full p-4 sm:p-6 animate-in fade-in zoom-in-95 duration-200 max-h-[90vh] overflow-y-auto"
      style="{maxWidthStyles[size] || 'max-width: 460px;'} width: 100%;"
    >
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-base font-semibold font-outfit text-white">{title}</h3>
        <button
          type="button"
          onclick={onclose}
          style="width: 32px; height: 32px; border-radius: 50%; display: flex; align-items: center; justify-content: center; background: transparent; border: none; outline: none; color: var(--text-muted); cursor: pointer; transition: color 200ms, background 200ms;"
          onmouseenter={(e) => { e.currentTarget.style.color = 'var(--text-primary)'; e.currentTarget.style.background = 'var(--bg-card-hover)'; }}
          onmouseleave={(e) => { e.currentTarget.style.color = 'var(--text-muted)'; e.currentTarget.style.background = 'transparent'; }}
        >
          <IconClose style="width: 20px; height: 20px;" />
        </button>
      </div>

      {#if children}
        <div class="modal-body">
          {@render children()}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-box {
    background: var(--bg-dropdown) !important;
    border: var(--border-width) solid var(--border-color) !important;
    border-radius: var(--radius-xl) !important;
    box-shadow: 0 20px 40px -15px rgba(0, 0, 0, 0.85) !important;
    box-sizing: border-box;
  }

  .modal-body {
    font-family: var(--font-sans);
    color: var(--text-primary);
  }
</style>
