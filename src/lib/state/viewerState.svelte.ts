import type { MediaViewerItem } from '$lib/components/content/MediaViewer.svelte';

export type ViewerMode = 'closed' | 'full' | 'docked';

export interface ViewerCallbacks {
  ondownload?: (item: MediaViewerItem, index: number) => void | Promise<void>;
  onopenpost?: (item: MediaViewerItem, index: number) => void;
  ondelete?: (item: MediaViewerItem, index: number) => void | Promise<void>;
}

export interface ViewerOpenInput extends ViewerCallbacks {
  items: MediaViewerItem[];
  index?: number;
  time?: number;
  originRect?: DOMRect | null;
}

/**
 * The viewer session outlives the page that opened it: the mini player can keep a video
 * running while the user navigates, so items are snapshotted and callbacks are detached
 * when their owner unmounts.
 */
class ViewerState {
  mode = $state<ViewerMode>('closed');
  items = $state<MediaViewerItem[]>([]);
  index = $state(0);
  initialTime = $state(0);
  originRect = $state<DOMRect | null>(null);
  callbacks = $state<ViewerCallbacks>({});

  private token = 0;

  current = $derived(this.items[this.index]);
  currentKey = $derived(this.current?.id || this.current?.url || '');
  isOpen = $derived(this.mode !== 'closed' && this.items.length > 0);

  open(input: ViewerOpenInput): number {
    if (input.items.length === 0) return this.token;
    this.items = [...input.items];
    this.index = Math.max(0, Math.min(input.items.length - 1, input.index ?? 0));
    this.initialTime = input.time && input.time > 0 ? input.time : 0;
    this.originRect = input.originRect ?? null;
    this.callbacks = {
      ondownload: input.ondownload,
      onopenpost: input.onopenpost,
      ondelete: input.ondelete
    };
    this.mode = 'full';
    this.token += 1;
    return this.token;
  }

  /** A page going away must not leave its closures driving a docked session. */
  detachCallbacks(token?: number) {
    if (token !== undefined && token !== this.token) return;
    this.callbacks = {};
  }

  setIndex(next: number) {
    if (this.items.length === 0) return;
    this.index = Math.max(0, Math.min(this.items.length - 1, next));
  }

  setItems(items: MediaViewerItem[], index?: number) {
    if (items.length === 0) {
      this.close();
      return;
    }
    this.items = [...items];
    this.index = Math.max(0, Math.min(items.length - 1, index ?? this.index));
  }

  removeAt(index: number) {
    if (index < 0 || index >= this.items.length) return;
    const next = this.items.filter((_, i) => i !== index);
    if (next.length === 0) {
      this.close();
      return;
    }
    this.items = next;
    this.index = Math.max(0, Math.min(next.length - 1, this.index >= index ? this.index - 1 : this.index));
  }

  dock() {
    if (this.mode !== 'full') return;
    this.mode = 'docked';
  }

  expand() {
    if (this.mode !== 'docked') return;
    this.initialTime = 0;
    this.mode = 'full';
  }

  close() {
    this.mode = 'closed';
    this.items = [];
    this.index = 0;
    this.initialTime = 0;
    this.originRect = null;
    this.callbacks = {};
    this.token += 1;
  }
}

export const viewerState = new ViewerState();
