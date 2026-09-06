<script lang="ts">
  import { untrack } from 'svelte';
  import { i18n } from '$lib/i18n';
  import Button from './Button.svelte';
  import IconAdd from '~icons/fluent/add-24-regular';
  import IconSubtract from '~icons/fluent/subtract-24-regular';

  interface Props {
    size?: 'sm' | 'base' | 'md' | 'lg';
    value: number;
    min?: number;
    max?: number;
    step?: number;
    ariaLabel?: string;
    decrementLabel?: string;
    incrementLabel?: string;
    onchange?: (value: number) => void;
    class?: string;
  }

  let {
    value,
    min = Number.NEGATIVE_INFINITY,
    max = Number.POSITIVE_INFINITY,
    step = 1,
    ariaLabel,
    decrementLabel,
    incrementLabel,
    onchange,
    size,
    class: extraClass = ''
  }: Props = $props();

  let finalDecrementLabel = $derived(decrementLabel ?? (i18n.t('common.decrease') || 'Decrease'));
  let finalIncrementLabel = $derived(incrementLabel ?? (i18n.t('common.increase') || 'Increase'));

  let isEditing = $state(false);
  let draftValue = $state(String(untrack(() => value)));

  let canDecrement = $derived(value > min);
  let canIncrement = $derived(value < max);

  $effect(() => {
    if (!isEditing) draftValue = String(value);
  });

  function decimalPlaces(number: number) {
    const [, decimals = ''] = String(number).split('.');
    return decimals.length;
  }

  function normalize(candidate: number) {
    const precision = Math.max(decimalPlaces(step), decimalPlaces(min), decimalPlaces(max));
    const factor = 10 ** precision;
    const base = Number.isFinite(min) ? min : 0;
    const stepped = base + Math.round((candidate - base) / step) * step;
    return Math.min(max, Math.max(min, Math.round(stepped * factor) / factor));
  }

  function parsedDraft() {
    return Number(draftValue.trim().replace(',', '.'));
  }

  function emit(nextValue: number) {
    draftValue = String(nextValue);
    if (nextValue !== value) onchange?.(nextValue);
  }

  function commit() {
    const parsed = parsedDraft();
    if (Number.isFinite(parsed)) emit(normalize(parsed));
    else draftValue = String(value);
    isEditing = false;
  }

  function cancel(input: HTMLInputElement) {
    draftValue = String(value);
    isEditing = false;
    input.blur();
  }

  function change(direction: -1 | 1) {
    const parsed = parsedDraft();
    const currentValue = Number.isFinite(parsed) ? normalize(parsed) : value;
    emit(normalize(currentValue + direction * step));
  }

  function handleInputKeydown(event: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (event.key === 'Enter') {
      event.preventDefault();
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      cancel(event.currentTarget);
    } else if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      change(event.key === 'ArrowUp' ? 1 : -1);
      event.currentTarget.select();
    }
  }
</script>

<div class="number-stepper {size ? `size-${size}` : ''} {extraClass}" role="group" aria-label={ariaLabel}>
  <Button
    variant="ghost"
    class="number-stepper__button"
    disabled={!canDecrement}
    aria-label={finalDecrementLabel}
    onclick={() => change(-1)}
  >
    <IconSubtract aria-hidden="true" />
  </Button>

  <input
    class="number-stepper__input"
    type="text"
    inputmode={Number.isInteger(step) ? 'numeric' : 'decimal'}
    role="spinbutton"
    aria-label={ariaLabel}
    aria-valuenow={value}
    aria-valuemin={Number.isFinite(min) ? min : undefined}
    aria-valuemax={Number.isFinite(max) ? max : undefined}
    value={draftValue}
    oninput={(event) => (draftValue = event.currentTarget.value)}
    onfocus={(event) => {
      isEditing = true;
      event.currentTarget.select();
    }}
    onblur={commit}
    onkeydown={handleInputKeydown}
  />

  <Button
    variant="ghost"
    class="number-stepper__button"
    disabled={!canIncrement}
    aria-label={finalIncrementLabel}
    onclick={() => change(1)}
  >
    <IconAdd aria-hidden="true" />
  </Button>
</div>

<style>
  .number-stepper {
    display: inline-grid;
    grid-template-columns:
      calc(var(--control-height, 34px) * var(--ui-scale, 1))
      calc(44px * var(--ui-scale, 1))
      calc(var(--control-height, 34px) * var(--ui-scale, 1));
    align-items: center;
    width: calc((var(--control-height, 34px) * 2 + 44px) * var(--ui-scale, 1));
    max-width: 100%;
    height: calc(var(--control-height, 34px) * var(--ui-scale, 1));
    border: 0;
    border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
    background: var(--select-bg, var(--bg-card-hover));
    overflow: clip;
    box-sizing: border-box;
  }

  .number-stepper.size-sm {
    --control-height: var(--control-height-sm, 34px);
    --control-font-size: var(--control-font-sm, 12px);
    --control-icon-size: var(--control-icon-sm, 16px);
  }

  .number-stepper.size-base {
    --control-height: var(--control-height-base, 40px);
    --control-font-size: var(--control-font-base, 13.5px);
    --control-icon-size: var(--control-icon-base, 18px);
  }

  .number-stepper.size-md {
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
  }

  .number-stepper.size-lg {
    --control-height: var(--control-height-lg, 52px);
    --control-font-size: var(--control-font-lg, 15px);
    --control-icon-size: var(--control-icon-lg, 22px);
  }

  :global(.number-stepper__button.btn) {
    width: calc(var(--control-height, 34px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 34px) * var(--ui-scale, 1)) !important;
    height: 100% !important;
    padding: 0 !important;
    border: 0 !important;
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1)) !important;
    background: transparent;
    color: var(--text-secondary);
    font-size: calc(var(--control-font-size, 16px) * var(--ui-scale, 1));
    font-weight: var(--font-weight-normal);
    line-height: 0;
  }

  :global(.number-stepper__button.btn:hover) {
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }

  :global(.number-stepper__button.btn svg) {
    display: block;
    width: calc(var(--control-icon-size, 16px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-icon-size, 16px) * var(--ui-scale, 1)) !important;
    margin: 0 !important;
  }

  .number-stepper__input {
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0 calc(2px * var(--ui-scale, 1)) calc(1px * var(--ui-scale, 1));
    border: 0;
    border-radius: calc(var(--radius-xs) * var(--ui-scale, 1));
    outline: 0;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: calc(13px * var(--ui-scale, 1));
    font-weight: var(--font-weight-normal);
    font-variant-numeric: tabular-nums;
    line-height: normal;
    text-align: center;
    white-space: nowrap;
    box-sizing: border-box;
    appearance: textfield;
  }

  @media (max-width: 640px) {
    .number-stepper {
      grid-template-columns:
        calc(var(--control-height, 34px) * var(--ui-scale, 1))
        minmax(0, 1fr)
        calc(var(--control-height, 34px) * var(--ui-scale, 1));
      width: 100%;
    }
  }
</style>
