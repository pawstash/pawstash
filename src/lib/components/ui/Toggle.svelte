<script lang="ts">
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
  role="switch"
  aria-checked={checked}
  aria-label={ariaLabel}
  {disabled}
  onclick={toggle}
  class="switch-root {extraClass}"
  class:checked
  class:disabled
>
  <span class="switch-thumb"></span>
</button>

<style>
  .switch-root {
    --track-inner: calc(
      (var(--switch-track-width) - var(--switch-outline-width) * 2) * var(--ui-scale, 1)
    );
    --thumb-off: calc(var(--switch-thumb-off) * var(--ui-scale, 1));
    --thumb-on: calc(var(--switch-thumb-on) * var(--ui-scale, 1));
    --inset-off: calc(
      ((var(--switch-track-height) - var(--switch-thumb-off)) / 2 - var(--switch-outline-width)) *
        var(--ui-scale, 1)
    );
    --inset-on: calc(
      ((var(--switch-track-height) - var(--switch-thumb-on)) / 2 - var(--switch-outline-width)) *
        var(--ui-scale, 1)
    );

    position: relative;
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    width: calc(var(--switch-track-width) * var(--ui-scale, 1));
    min-width: calc(var(--switch-track-width) * var(--ui-scale, 1));
    max-width: calc(var(--switch-track-width) * var(--ui-scale, 1));
    height: calc(var(--switch-track-height) * var(--ui-scale, 1));
    min-height: calc(var(--switch-track-height) * var(--ui-scale, 1));
    padding: 0;
    border: calc(var(--switch-outline-width) * var(--ui-scale, 1)) solid
      var(--accent-outline);
    border-radius: var(--radius-full);
    background: var(--surface-container-highest);
    cursor: pointer;
    box-sizing: border-box;
    overflow: hidden;
    transition:
      background var(--duration-fast) var(--ease-expo),
      border-color var(--duration-fast) var(--ease-expo);
  }

  .switch-thumb {
    position: absolute;
    top: 50%;
    left: var(--inset-off);
    right: calc(var(--track-inner) - var(--inset-off) - var(--thumb-off));
    height: var(--thumb-off);
    border-radius: var(--radius-full);
    background: var(--accent-outline);
    transform: translateY(-50%);
    pointer-events: none;
    transition:
      left var(--duration-fast) var(--ease-modal-spring),
      right var(--duration-normal) var(--ease-trail),
      height var(--duration-normal) var(--ease-expo),
      background var(--duration-fast) var(--ease-expo);
  }

  .switch-root.checked .switch-thumb {
    left: calc(var(--track-inner) - var(--inset-on) - var(--thumb-on));
    right: var(--inset-on);
    height: var(--thumb-on);
    background: var(--accent-on-primary);
    transition:
      left var(--duration-normal) var(--ease-trail),
      right var(--duration-fast) var(--ease-modal-spring),
      height var(--duration-normal) var(--ease-expo),
      background var(--duration-fast) var(--ease-expo);
  }

  .switch-root.checked {
    border-color: var(--accent-primary);
    background: var(--accent-primary);
  }

  .switch-root:active:not(.disabled):not(.checked) .switch-thumb {
    right: calc(var(--track-inner) - var(--inset-off) - var(--thumb-off) - var(--thumb-off) / 2);
  }

  .switch-root.checked:active:not(.disabled) .switch-thumb {
    left: calc(
      var(--track-inner) - var(--inset-on) - var(--thumb-on) - var(--thumb-on) / 2
    );
  }

  .switch-root:hover:not(.disabled):not(.checked) {
    background: var(--bg-card-hover);
  }

  .switch-root:hover:not(.disabled).checked {
    border-color: var(--accent-primary-hover);
    background: var(--accent-primary-hover);
  }

  .switch-root:focus-visible {
    outline: calc(2px * var(--ui-scale, 1)) solid var(--accent-primary) !important;
    outline-offset: calc(2px * var(--ui-scale, 1)) !important;
  }

  .switch-root.disabled {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }

  @media (prefers-reduced-motion: reduce) {
    .switch-root,
    .switch-thumb,
    .switch-root.checked .switch-thumb {
      transition: none;
    }
  }
</style>
