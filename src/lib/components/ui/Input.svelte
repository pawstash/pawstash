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
    size?: 'sm' | 'base' | 'md' | 'lg';
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
    size,
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
    if (ref) {
      ref.value = '';
      ref.dispatchEvent(new Event('input', { bubbles: true }));
      ref.dispatchEvent(new Event('change', { bubbles: true }));
    } else {
      const syntheticTarget = { value: '' };
      oninput?.({ target: syntheticTarget, currentTarget: syntheticTarget } as unknown as Event);
      onchange?.({ target: syntheticTarget, currentTarget: syntheticTarget } as unknown as Event);
    }
  }
</script>

<div
  class="input-box {size ? `size-${size}` : ''} {extraClass}"
  class:is-disabled={disabled}
  class:is-readonly={readonly}
>
  {#if IconComponent}
    <div class="left-icon" aria-hidden="true">
      <IconComponent />
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
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1));
    padding: 0 calc(var(--control-padding-x, 14px) * var(--ui-scale, 1));
    gap: calc(8px * var(--ui-scale, 1));
    background: var(--input-bg, rgba(255, 255, 255, 0.06));
    border: none;
    border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
    box-sizing: border-box;
    font-size: calc(var(--control-font-size, 14px) * var(--ui-scale, 1));
    font-family: var(--font-sans);
    line-height: normal;
    color: var(--text-primary);
    outline: none !important;
    box-shadow: none !important;
    transition: background var(--duration-fast) var(--ease-expo);
  }

  .input-box.size-sm {
    --control-height: var(--control-height-sm, 34px);
    --control-font-size: var(--control-font-sm, 12.5px);
    --control-icon-size: var(--control-icon-sm, 16px);
    --control-padding-x: var(--control-padding-sm, 12px);
  }

  .input-box.size-base {
    --control-height: var(--control-height-base, 40px);
    --control-font-size: var(--control-font-base, 13.5px);
    --control-icon-size: var(--control-icon-base, 18px);
    --control-padding-x: var(--control-padding-base, 14px);
  }

  .input-box.size-md {
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
    --control-padding-x: var(--control-padding-md, 16px);
  }

  .input-box.size-lg {
    --control-height: var(--control-height-lg, 52px);
    --control-font-size: var(--control-font-lg, 15px);
    --control-icon-size: var(--control-icon-lg, 22px);
    --control-padding-x: var(--control-padding-lg, 20px);
  }

  .input-box :global(.left-icon svg) {
    width: calc(var(--control-icon-size, 19px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-icon-size, 19px) * var(--ui-scale, 1)) !important;
  }

  .input-box :global(.icon-btn svg) {
    width: calc(var(--control-icon-size, 18px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-icon-size, 18px) * var(--ui-scale, 1)) !important;
  }

  .input-box:hover,
  .input-box:focus-within {
    background: var(--input-bg-hover, rgba(255, 255, 255, 0.095));
  }

  .input-box:focus-visible,
  .input-box:has(:focus-visible) {
    outline: calc(1.5px * var(--ui-scale, 1)) solid var(--accent-primary) !important;
    outline-offset: calc(-1.5px * var(--ui-scale, 1)) !important;
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
    line-height: normal;
    box-sizing: border-box;
  }

  .native-input:focus,
  .native-input:focus-visible {
    outline: none !important;
    border: none !important;
    box-shadow: none !important;
  }

  .native-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.85;
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
