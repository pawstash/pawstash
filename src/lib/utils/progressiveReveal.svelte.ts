const FIRST_BATCH = 16;
const BATCH = 16;

export function progressiveReveal(total: () => number, initial = FIRST_BATCH) {
  let revealed = $state(initial);

  $effect(() => {
    const count = total();
    if (revealed >= count) return;
    const frame = requestAnimationFrame(() => {
      revealed = Math.min(revealed + BATCH, count);
    });
    return () => cancelAnimationFrame(frame);
  });

  return {
    get revealed() {
      return revealed;
    }
  };
}
