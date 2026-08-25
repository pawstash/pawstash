<script lang="ts">
  import IconCheck from '~icons/fluent/checkmark-24-filled';

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
    class?: string;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    onchange,
    class: extraClass = ''
  }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    if (onchange) onchange(checked);
  }
</script>

<button
  type="button"
  role="checkbox"
  aria-checked={checked}
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
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    outline: none;
    user-select: none;
    flex-shrink: 0;
  }

  .checkbox-box {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 6px;
    border: none;
    background: rgba(255, 255, 255, 0.03);
    color: #ffffff;
    transition: background 0.18s cubic-bezier(0.16, 1, 0.3, 1),
                transform 0.1s ease;
  }

  .checkbox-root:hover:not(.disabled) .checkbox-box {
    background: rgba(255, 255, 255, 0.08);
  }

  .checkbox-root:active:not(.disabled) .checkbox-box {
    transform: scale(0.95);
  }

  .checkbox-root.checked .checkbox-box {
    background: rgba(255, 255, 255, 0.08);
    color: #ffffff;
  }

  .checkbox-root.checked:hover:not(.disabled) .checkbox-box {
    background: rgba(255, 255, 255, 0.12);
  }

  :global(.checkbox-icon) {
    width: 17px !important;
    height: 17px !important;
    color: #ffffff !important;
    animation: checkIn 0.18s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  .checkbox-root.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @keyframes checkIn {
    from {
      opacity: 0;
      transform: scale(0.6);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
