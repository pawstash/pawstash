import { apiGetAxumPort } from '$lib/utils/ipc';

export class ServerPortState {
  port = $state<number>(0);
  private isInitializing = false;

  async init(): Promise<number> {
    if (this.port > 0) return this.port;
    if (this.isInitializing) return this.port;
    this.isInitializing = true;

    try {
      for (let attempt = 0; attempt < 25; attempt++) {
        const p = await apiGetAxumPort().catch(() => 0);
        if (p > 0) {
          this.port = p;
          return p;
        }
        await new Promise((resolve) => setTimeout(resolve, 200));
      }
      return 0;
    } finally {
      this.isInitializing = false;
    }
  }

  async ensurePort(): Promise<number> {
    if (this.port > 0) return this.port;
    return this.init();
  }
}

export const serverPortState = new ServerPortState();

if (typeof window !== 'undefined') {
  void serverPortState.init();
}
