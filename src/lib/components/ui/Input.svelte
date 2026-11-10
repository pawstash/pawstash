<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { ripple, tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconEye from '~icons/fluent/eye-24-regular';
  import IconEyeOff from '~icons/fluent/eye-off-24-regular';

  interface Props {
    value?: string;
    label?: string;
    required?: boolean;
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
    labelledby?: string;
    describedby?: string;
    ref?: HTMLInputElement | null;
    size?: 'sm' | 'base' | 'md' | 'lg';
    class?: string;
  }

  let {
    value = $bindable(''),
    label,
    required = false,
    placeholder = '',
    type = 'text',
    autocomplete,
    name,
    disabled = false,
    readonly = false,
    clearable = false,
    autofocus = false,
    labelledby,
    describedby,
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
  let inputId = $props.id();
  let filled = $derived(value.length > 0);

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
  class:has-label={Boolean(label)}
  class:is-filled={filled}
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

  <div class="field-main">
    {#if label}
      <label class="floating-label" for={inputId}>
        {label}
        {#if required}<span class="floating-label__required" aria-hidden="true">*</span>{/if}
      </label>
    {/if}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      bind:this={ref}
      id={inputId}
      type={effectiveType}
      {placeholder}
      {disabled}
      {readonly}
      {autocomplete}
      {name}
      {autofocus}
      aria-labelledby={labelledby}
      aria-describedby={describedby}
      aria-required={required || undefined}
      bind:value
      {oninput}
      {onchange}
      {onblur}
      {onkeydown}
      class="native-input"
    />
  </div>

  <div class="right-actions">
    {#if clearable && value && !disabled && !readonly}
      <button
        type="button"
        class="field-action"
        use:ripple
        use:tooltip={i18n.t('common.clear_input')}
        onclick={handleClear}
        aria-label={i18n.t('common.clear_input')}
      >
        <IconDismiss />
      </button>
    {/if}

    {#if type === 'password' && !disabled}
      <button
        type="button"
        class="field-action"
        use:ripple
        use:tooltip={i18n.t(isPasswordVisible ? 'common.hide_password' : 'common.show_password')}
        onclick={(e) => {
          e.stopPropagation();
          isPasswordVisible = !isPasswordVisible;
        }}
        aria-label={i18n.t(isPasswordVisible ? 'common.hide_password' : 'common.show_password')}
      >
        {#if isPasswordVisible}
          <IconEyeOff />
        {:else}
          <IconEye />
        {/if}
      </button>
    {/if}

    {#if onBrowse && !disabled}
      <button
        type="button"
        class="field-action"
        use:ripple
        use:tooltip={i18n.t('common.browse')}
        onclick={(e) => {
          e.stopPropagation();
          onBrowse();
        }}
        aria-label={i18n.t('common.browse')}
      >
        <IconFolder />
      </button>
    {/if}

    {#if ActionIconComponent && onAction && !disabled}
      <button
        type="button"
        class="field-action"
        use:ripple
        use:tooltip={actionTooltip || ''}
        onclick={(e) => {
          e.stopPropagation();
          onAction();
        }}
        aria-label={actionTooltip}
      >
        <ActionIconComponent />
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

  .input-box.has-label {
    height: calc(56px * var(--ui-scale, 1));
  }

  .field-main {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
    height: 100%;
  }

  .input-box.has-label .native-input {
    padding-top: calc(16px * var(--ui-scale, 1));
  }

  .floating-label {
    position: absolute;
    inset-inline-start: 0;
    top: 50%;
    max-width: 100%;
    overflow: hidden;
    color: var(--text-secondary);
    font-size: calc(15px * var(--ui-scale, 1));
    line-height: normal;
    text-overflow: ellipsis;
    white-space: nowrap;
    pointer-events: none;
    transform: translateY(-50%);
    transform-origin: left center;
    transition:
      top var(--duration-fast) var(--ease-standard),
      transform var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-standard);
  }

  .input-box.has-label:focus-within .floating-label,
  .input-box.has-label.is-filled .floating-label {
    top: calc(15px * var(--ui-scale, 1));
    transform: translateY(-50%) scale(0.8);
  }

  .input-box.has-label:focus-within .floating-label {
    color: var(--accent-primary);
  }

  .input-box.has-label:not(:focus-within):not(.is-filled) .native-input::placeholder {
    color: transparent;
  }

  .floating-label__required {
    margin-inline-start: 2px;
    color: var(--status-error);
  }

  @media (prefers-reduced-motion: reduce) {
    .floating-label {
      transition: none;
    }
  }

  .input-box :global(.left-icon svg) {
    width: calc(var(--control-icon-size, 19px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-icon-size, 19px) * var(--ui-scale, 1)) !important;
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
    color: var(--accent-primary);
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
</style>
