<script lang="ts">
  import type { Snippet } from 'svelte';
  import { scrollable } from '$lib/actions/scrollable';
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

  const maxWidths: Record<string, string> = {
    sm: '380px',
    md: '460px',
    lg: '560px',
    xl: '680px'
  };
</script>

{#if isOpen}
  <div class="modal-overlay">
    <div
      role="button"
      tabindex="0"
      class="modal-backdrop"
      onclick={onclose}
      onkeydown={(e) => (e.key === 'Escape' || e.key === 'Enter') && onclose()}
    ></div>

    <div
      class="modal-box"
      style:max-width={maxWidths[size] || '460px'}
    >
      <div class="modal-header">
        <h3 class="modal-title">{title}</h3>
        <button
          type="button"
          class="modal-close-btn"
          onclick={onclose}
          aria-label="Close"
        >
          <IconClose class="w-5 h-5" />
        </button>
      </div>

      {#if children}
        <div class="modal-body-wrapper" use:scrollable>
          <div class="modal-body">
            {@render children()}
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    transition: opacity var(--duration-normal, 200ms) ease;
  }

  .modal-box {
    position: relative;
    z-index: 10;
    width: 100%;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-dropdown);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-xl);
    box-shadow: 0 24px 48px -12px rgba(0, 0, 0, 0.85);
    overflow: hidden;
    animation: modal-enter 200ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes modal-enter {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px 16px;
    flex-shrink: 0;
  }

  .modal-title {
    margin: 0;
    font-family: var(--font-outfit, var(--font-sans));
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .modal-close-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-full);
    display: grid;
    place-items: center;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: color var(--duration-fast, 150ms), background var(--duration-fast, 150ms);
  }

  .modal-close-btn:hover {
    color: var(--text-primary);
    background: var(--bg-card-hover);
  }

  .modal-body-wrapper {
    flex: 1 1 auto;
    min-height: 0;
    width: 100%;
  }

  .modal-body {
    padding: 0 24px 24px;
    font-family: var(--font-sans);
    color: var(--text-primary);
  }
</style>
