<script lang="ts">
  interface Props {
    checked?: boolean;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
  }

  let { checked = $bindable(false), disabled = false, onchange }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    if (onchange) onchange(checked);
  }
</script>

<button
  type="button"
  role="switch"
  aria-label="Toggle switch"
  aria-checked={checked}
  {disabled}
  onclick={toggle}
  class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none disabled:opacity-50 {checked ? 'bg-purple-600' : 'bg-white/10'}"
>
  <span
    class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-lg ring-0 transition duration-200 ease-in-out {checked ? 'translate-x-5' : 'translate-x-0'}"
  ></span>
</button>
