export type TriState = 'neutral' | 'include' | 'exclude';
export type TriStateFilter = TriState;
export type TriStateBool = 'all' | 'include' | 'exclude';
export type FilterMap = Record<string, TriState>;

export function nextTriState(current?: TriState): TriState {
  if (!current || current === 'neutral') return 'include';
  if (current === 'include') return 'exclude';
  return 'neutral';
}

export function nextTriStateBool(current?: TriStateBool): TriStateBool {
  if (!current || current === 'all') return 'include';
  if (current === 'include') return 'exclude';
  return 'all';
}

export function toggleFilterKey(map: FilterMap, key: string): FilterMap {
  const next = nextTriState(map[key]);
  const updated = { ...map };
  if (next === 'neutral') {
    delete updated[key];
  } else {
    updated[key] = next;
  }
  return updated;
}

export function getIncludedKeys(map: FilterMap): string[] {
  return Object.keys(map).filter((k) => map[k] === 'include');
}

export function getExcludedKeys(map: FilterMap): string[] {
  return Object.keys(map).filter((k) => map[k] === 'exclude');
}

/**
 * Checks whether an item's values match a tri-state filter map.
 * - If item matches any excluded key -> returns false.
 * - If there are included keys and item matches none of them -> returns false.
 * - Otherwise returns true.
 */
export function matchesTriStateFilter(itemValues: string[], filterMap: FilterMap): boolean {
  const excluded = getExcludedKeys(filterMap);
  if (excluded.length > 0 && excluded.some((ex) => itemValues.includes(ex))) {
    return false;
  }

  const included = getIncludedKeys(filterMap);
  if (included.length > 0 && !included.some((inc) => itemValues.includes(inc))) {
    return false;
  }

  return true;
}

/**
 * Checks a boolean flag against a TriStateBool.
 */
export function matchesTriStateBool(hasFlag: boolean, state: TriStateBool): boolean {
  if (state === 'include') return hasFlag;
  if (state === 'exclude') return !hasFlag;
  return true;
}

/**
 * Calculates the number of active (non-neutral) filters.
 */
export function countActiveFilters(
  filterMaps: FilterMap[],
  boolFilters: TriStateBool[] = []
): number {
  let count = 0;
  for (const map of filterMaps) {
    count += Object.values(map).filter((v) => v === 'include' || v === 'exclude').length;
  }
  for (const b of boolFilters) {
    if (b === 'include' || b === 'exclude') count++;
  }
  return count;
}
