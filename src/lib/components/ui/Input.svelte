<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { ripple, tooltip } from '$lib/motion';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconEye from '~icons/fluent/eye-24-regular';
  import IconEyeOff from '~icons/fluent/eye-off-24-regular';

  interface Props {
    value?: string;
    placeholder?: string;
    type?: string;
    autocomplete?: 'username' | 'current-password' | 'new-password' | 'off';
    name?: string;
    disabled?: boolean;
    readonly?: boolean;
    clearable?: boolean;
    icon?: Component;
    left?: Snippet;
    right?: Snippet;
    actionIcon?: Component;
    actionTooltip?: string;
    onAction?: () => void;
    onBrowse?: () => void;
    oninput?: (e: Event) => void;
    onchange?: (e: Event) => void;
    onblur?: (e: Event) => void;
    onkeydown?: (e: KeyboardEvent) => void;
    autofocus?: boolean;
    ref?: HTMLInputElement | null;
    class?: string;
  }

  let {
    value = $bindable(''),
    placeholder = '',
    type = 'text',
    autocomplete,
    name,
    disabled = false,
    readonly = false,
    clearable = false,
    autofocus = false,
    ref = $bindable(),
    icon: IconComponent,
    left,
    right,
    actionIcon: ActionIconComponent,
    actionTooltip,
    onAction,
    onBrowse,
    oninput,
    onchange,
    onblur,
    onkeydown,
    class: extraClass = ''
  }: Props = $props();

  let isPasswordVisible = $state(false);
  let effectiveType = $derived(
    type === 'password' ? (isPasswordVisible ? 'text' : 'password') : type
  );

  function handleClear(e: MouseEvent) {
    e.stopPropagation();
    value = '';
    oninput?.(new Event('input'));
    onchange?.(new Event('change'));
  }
</script>

<div
  class="input-box {extraClass}"
  class:is-disabled={disabled}
  class:is-readonly={readonly}
>
  {#if IconComponent}
    <div class="left-icon" aria-hidden="true">
      <IconComponent style="width: 19px; height: 19px;" />
    </div>
  {:else if left}
    <div class="left-icon">
      {@render left()}
    </div>
  {/if}

  <!-- svelte-ignore a11y_autofocus -->
  <input
    bind:this={ref}
    type={effectiveType}
    {placeholder}
    {disabled}
    {readonly}
    {autocomplete}
    {name}
    {autofocus}
    bind:value
    {oninput}
    {onchange}
    {onblur}
    {onkeydown}
    class="native-input"
  />

  <div class="right-actions">
    {#if clearable && value && !disabled && !readonly}
      <button
        type="button"
        class="icon-btn"
        use:ripple
        use:tooltip={'Clear'}
        onclick={handleClear}
        aria-label="Clear input"
      >
        <IconDismiss style="width: 18px; height: 18px;" />
      </button>
    {/if}

    {#if type === 'password' && !disabled}
      <button
        type="button"
        class="icon-btn"
        use:ripple
        use:tooltip={isPasswordVisible ? 'Hide password' : 'Show password'}
        onclick={(e) => {
          e.stopPropagation();
          isPasswordVisible = !isPasswordVisible;
        }}
        aria-label={isPasswordVisible ? 'Hide password' : 'Show password'}
      >
        {#if isPasswordVisible}
          <IconEyeOff style="width: 18px; height: 18px;" />
        {:else}
          <IconEye style="width: 18px; height: 18px;" />
        {/if}
      </button>
    {/if}

    {#if onBrowse && !disabled}
      <button
        type="button"
        class="icon-btn"
        use:ripple
        use:tooltip={'Browse'}
        onclick={(e) => {
          e.stopPropagation();
          onBrowse();
        }}
        aria-label="Browse folder"
      >
        <IconFolder style="width: 18px; height: 18px;" />
      </button>
    {/if}

    {#if ActionIconComponent && onAction && !disabled}
      <button
        type="button"
        class="icon-btn"
        use:ripple
        use:tooltip={actionTooltip || ''}
        onclick={(e) => {
          e.stopPropagation();
          onAction();
        }}
        aria-label={actionTooltip || 'Action'}
      >
        <ActionIconComponent style="width: 18px; height: 18px;" />
      </button>
    {/if}

    {#if right}
      {@render right()}
    {/if}
  </div>
</div>

<style>
  .input-box {
    display: flex;
    align-items: center;
    width: 100%;
    height: 46px;
    padding: 0 14px;
    gap: 10px;
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    box-sizing: border-box;
    font-size: 14px;
    font-family: var(--font-sans);
    color: var(--text-primary);
    outline: none !important;
    box-shadow: none !important;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo);
  }

  .input-box:hover,
  .input-box:focus-within {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
  }

  .input-box:focus-visible,
  .input-box:has(:focus-visible) {
    border-color: var(--border-color-focus);
    outline: none !important;
    box-shadow: none !important;
  }

  .input-box.is-disabled {
    opacity: 0.5;
    pointer-events: none;
  }

  .left-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    opacity: 0.65;
    flex-shrink: 0;
    transition: color var(--duration-fast), opacity var(--duration-fast);
  }

  .input-box:hover .left-icon,
  .input-box:focus-within .left-icon {
    color: var(--accent);
    opacity: 1;
  }

  .native-input {
    flex: 1;
    min-width: 0;
    height: 100%;
    background: transparent !important;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
    padding: 0;
    color: var(--text-primary);
    font-size: inherit;
    font-family: inherit;
    box-sizing: border-box;
  }

  .native-input:focus,
  .native-input:focus-visible {
    outline: none !important;
    border: none !important;
    box-shadow: none !important;
  }

  .native-input::placeholder {
    color: var(--text-muted);
    opacity: 0.6;
    font-size: 13px;
  }

  .right-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    margin-left: auto;
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    border: none;
    outline: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    opacity: 0.45;
    cursor: pointer;
    padding: 0;
    border-radius: var(--radius-sm, 6px);
    transition: opacity 160ms ease, color 160ms ease;
  }

  .icon-btn:hover,
  .icon-btn:focus-visible {
    opacity: 0.95;
    color: var(--accent);
  }
</style>
