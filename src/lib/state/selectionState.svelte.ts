export type SelectionScope = 'posts' | 'creators' | 'downloads';

export class SelectionState {
  active = $state(false);
  scope = $state<SelectionScope | null>(null);
  selectedKeys = $state<Set<string>>(new Set());
  selectedItems = $state<Map<string, any>>(new Map());
  lastSelectedKey = $state<string | null>(null);
  registeredKeys = $state<string[]>([]);
  registeredItemsMap = $state<Map<string, any>>(new Map());

  get count() {
    return this.selectedKeys.size;
  }

  isSelected(key: string) {
    return this.selectedKeys.has(key);
  }

  setContext(scope: SelectionScope, keys: string[], itemsMap?: Map<string, any>) {
    if (this.scope === null || this.scope === scope) {
      this.registeredKeys = keys;
      if (itemsMap) {
        this.registeredItemsMap = itemsMap;
      }
    }
  }

  enter(scope: SelectionScope, initialKey?: string, initialItem?: any) {
    this.scope = scope;
    this.active = true;
    if (initialKey) {
      this.selectedKeys = new Set([initialKey]);
      this.selectedItems = new Map(initialItem ? [[initialKey, initialItem]] : []);
      this.lastSelectedKey = initialKey;
    } else {
      this.selectedKeys = new Set();
      this.selectedItems = new Map();
      this.lastSelectedKey = null;
    }
  }

  exit() {
    this.active = false;
    this.scope = null;
    this.selectedKeys = new Set();
    this.selectedItems = new Map();
    this.lastSelectedKey = null;
  }

  clear() {
    this.selectedKeys = new Set();
    this.selectedItems = new Map();
    this.lastSelectedKey = null;
  }

  toggle(
    scope: SelectionScope,
    key: string,
    item?: any,
    orderedKeys?: string[],
    shiftKey = false,
    itemsMap?: Map<string, any>
  ) {
    if (!this.active || this.scope !== scope) {
      this.enter(scope, key, item);
      return;
    }

    const effectiveKeys = (orderedKeys && orderedKeys.length > 0) ? orderedKeys : this.registeredKeys;
    const effectiveItems = itemsMap || this.registeredItemsMap;

    if (shiftKey && this.lastSelectedKey && effectiveKeys.length > 0) {
      const fromIndex = effectiveKeys.indexOf(this.lastSelectedKey);
      const toIndex = effectiveKeys.indexOf(key);

      if (fromIndex !== -1 && toIndex !== -1) {
        const start = Math.min(fromIndex, toIndex);
        const end = Math.max(fromIndex, toIndex);
        const rangeKeys = effectiveKeys.slice(start, end + 1);

        const newKeys = new Set(this.selectedKeys);
        const newItems = new Map(this.selectedItems);

        for (const k of rangeKeys) {
          newKeys.add(k);
          if (effectiveItems.has(k)) {
            newItems.set(k, effectiveItems.get(k));
          }
        }
        if (item) newItems.set(key, item);

        this.selectedKeys = newKeys;
        this.selectedItems = newItems;
        this.lastSelectedKey = key;
        return;
      }
    }

    const newKeys = new Set(this.selectedKeys);
    const newItems = new Map(this.selectedItems);

    if (newKeys.has(key)) {
      newKeys.delete(key);
      newItems.delete(key);
      if (newKeys.size === 0) {
        this.lastSelectedKey = null;
      } else if (this.lastSelectedKey === key) {
        const keysArr = Array.from(newKeys);
        this.lastSelectedKey = keysArr[keysArr.length - 1] ?? null;
      }
    } else {
      newKeys.add(key);
      if (item) newItems.set(key, item);
      else if (effectiveItems.has(key)) newItems.set(key, effectiveItems.get(key));
      this.lastSelectedKey = key;
    }

    this.selectedKeys = newKeys;
    this.selectedItems = newItems;
  }

  select(key: string, item?: any) {
    const newKeys = new Set(this.selectedKeys).add(key);
    const newItems = new Map(this.selectedItems);
    if (item) newItems.set(key, item);
    this.selectedKeys = newKeys;
    this.selectedItems = newItems;
    this.lastSelectedKey = key;
  }

  deselect(key: string) {
    const newKeys = new Set(this.selectedKeys);
    newKeys.delete(key);
    const newItems = new Map(this.selectedItems);
    newItems.delete(key);
    this.selectedKeys = newKeys;
    this.selectedItems = newItems;
  }

  selectAll(items: { key: string; item: any }[]) {
    const newKeys = new Set<string>();
    const newItems = new Map<string, any>();
    for (const { key, item } of items) {
      newKeys.add(key);
      newItems.set(key, item);
    }
    this.selectedKeys = newKeys;
    this.selectedItems = newItems;
    if (items.length > 0) {
      this.lastSelectedKey = items[items.length - 1].key;
    }
  }

  getItems<T>(): T[] {
    return Array.from(this.selectedItems.values()) as T[];
  }
}

export const selectionState = new SelectionState();
