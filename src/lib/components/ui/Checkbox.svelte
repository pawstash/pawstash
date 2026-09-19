<script lang="ts">
  import IconCheck from '~icons/fluent/checkmark-24-filled';

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
    ariaLabel?: string;
    class?: string;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    onchange,
    ariaLabel,
    class: extraClass = ''
  }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<button
  type="button"
  role="checkbox"
  aria-checked={checked}
  aria-label={ariaLabel}
  {disabled}
  onclick={toggle}
  class="checkbox-root {extraClass}"
  class:checked
  class:disabled
>
  <span class="checkbox-box">
    {#if checked}
      <IconCheck class="checkbox-icon" />
    {/if}
  </span>
</button>

<style>
  .checkbox-root {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    user-select: none;
  }

  .checkbox-box {
    display: flex;
    align-items: center;
    justify-content: center;
    width: calc(var(--checkbox-size) * var(--ui-scale, 1));
    height: calc(var(--checkbox-size) * var(--ui-scale, 1));
    border: calc(var(--checkbox-outline-width) * var(--ui-scale, 1)) solid
      var(--accent-outline);
    border-radius: calc(var(--checkbox-radius) * var(--ui-scale, 1));
    background: transparent;
    color: var(--accent-on-primary);
    box-sizing: border-box;
    transition:
      background var(--duration-fast) var(--ease-expo),
      border-color var(--duration-fast) var(--ease-expo),
      transform var(--duration-fast) var(--ease-expo);
  }

  .checkbox-root:hover:not(.disabled) .checkbox-box {
    background: var(--bg-card-hover);
  }

  .checkbox-root:active:not(.disabled) .checkbox-box {
    transform: scale(0.94);
  }

  .checkbox-root.checked .checkbox-box {
    border-color: var(--accent-primary);
    background: var(--accent-primary);
  }

  .checkbox-root.checked:hover:not(.disabled) .checkbox-box {
    border-color: var(--accent-primary-hover);
    background: var(--accent-primary-hover);
  }

  .checkbox-root:focus-visible {
    outline: none !important;
  }

  .checkbox-root:focus-visible .checkbox-box {
    outline: calc(2px * var(--ui-scale, 1)) solid var(--accent-primary) !important;
    outline-offset: calc(2px * var(--ui-scale, 1)) !important;
  }

  .checkbox-root.disabled {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }

  :global(.checkbox-icon) {
    width: calc(var(--checkbox-icon-size) * var(--ui-scale, 1)) !important;
    height: calc(var(--checkbox-icon-size) * var(--ui-scale, 1)) !important;
    color: currentColor !important;
    animation: check-in var(--duration-fast) var(--ease-expo) forwards;
  }

  @media (pointer: coarse) {
    .checkbox-root::after {
      content: '';
      position: absolute;
      top: 50%;
      left: 50%;
      width: var(--tap-target-min);
      height: var(--tap-target-min);
      transform: translate(-50%, -50%);
    }
  }

  @keyframes check-in {
    from {
      opacity: 0;
      transform: scale(0.6);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .checkbox-box,
    :global(.checkbox-icon) {
      transition: none;
      animation: none;
    }
  }
</style>
