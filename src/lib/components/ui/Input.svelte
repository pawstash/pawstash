<script lang="ts">
  import IconFolder from '~icons/fluent/folder-24-regular';
  import { ripple } from '$lib/motion';

  interface Props {
    value?: string;
    placeholder?: string;
    type?: string;
    autocomplete?: 'username' | 'current-password' | 'new-password' | 'off';
    name?: string;
    disabled?: boolean;
    readonly?: boolean;
    oninput?: (e: Event) => void;
    onchange?: (e: Event) => void;
    onblur?: (e: Event) => void;
    onkeydown?: (e: KeyboardEvent) => void;
    onBrowse?: () => void;
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
    oninput,
    onchange,
    onblur,
    onkeydown,
    onBrowse,
    class: extraClass = ''
  }: Props = $props();
</script>

<!-- svelte-ignore css_unused_selector -->
<div style="position:relative; width:100%;">
  <input
    {type}
    {placeholder}
    {disabled}
    {readonly}
    {autocomplete}
    {name}
    bind:value
    {oninput}
    {onchange}
    {onblur}
    {onkeydown}
    class="input-field {extraClass}"
    style={onBrowse ? 'padding-right: 44px;' : ''}
  />
  {#if onBrowse}
    <button
      type="button"
      use:ripple
      onclick={onBrowse}
      style="position:absolute; right:14px; top:50%; transform:translateY(-50%); width:20px; height:20px; border:none; outline:none; background:transparent; display:flex; align-items:center; justify-content:center; color:white; opacity:0.45; cursor:pointer; z-index:10; padding:0; transition:opacity 200ms ease;"
      aria-label="Browse"
      onmouseenter={(e) => { e.currentTarget.style.opacity = '0.8'; }}
      onmouseleave={(e) => { e.currentTarget.style.opacity = '0.45'; }}
    >
      <IconFolder style="width:20px; height:20px;" />
    </button>
  {/if}
</div>

<style>
  .input-field {
    width: 100%;
    height: 46px;
    padding: 0 14px;
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-size: 14px;
    font-family: var(--font-sans);
    outline: none;
    box-sizing: border-box;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo),
                box-shadow var(--duration-fast) var(--ease-expo);
  }

  .input-field::placeholder {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .input-field:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
  }

  .input-field:focus {
    background: var(--bg-card-hover);
    border-color: var(--border-color-focus);
    box-shadow: 0 0 0 2px var(--accent-glow);
  }

  .input-field:disabled {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }
</style>
