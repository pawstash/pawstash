import { apiGetAxumPort } from '$lib/utils/ipc';

export class ServerPortState {
  port = $state<number>(0);
  token = $state<string>('');
  private isInitializing = false;

  async init(): Promise<number> {
    if (this.port > 0) return this.port;
    if (this.isInitializing) return this.port;
    this.isInitializing = true;

    try {
      for (let attempt = 0; attempt < 25; attempt++) {
        const info = await apiGetAxumPort().catch(() => ({ port: 0, token: '' }));
        if (info.port > 0 && info.token) {
          this.port = info.port;
          this.token = info.token;
          return info.port;
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

  mediaUrl(path: string): string {
    if (this.port <= 0 || !this.token) return '';
    const url = new URL(path, `http://127.0.0.1:${this.port}`);
    url.searchParams.set('token', this.token);
    return url.toString();
  }
}

export const serverPortState = new ServerPortState();

if (typeof window !== 'undefined') {
  void serverPortState.init();
}
