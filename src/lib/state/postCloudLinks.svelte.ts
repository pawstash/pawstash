import type { Attachment } from '$lib/types/content';
import type { CloudFolderResult, CloudLinkContext, CloudLockKind } from '$lib/types/cloud';
import { apiResolveCloudLink } from '$lib/utils/ipc';
import { cloudLockKind } from '$lib/utils/cloudLink';
import { cloudResultAttachments } from '$lib/utils/cloudNodes';
import { isSameAttachment } from '$lib/utils/media';
import { logger } from '$lib/utils/logger';

const folderCache = new Map<string, CloudFolderResult>();
const postNodesCache = new Map<string, Attachment[]>();

function missingSizes(nodes: Attachment[]): boolean {
  return nodes.some((node) => !node.is_cloud_folder && (!node.size || node.size === 0));
}

export class PostCloudLinks {
  resolved = $state<Attachment[]>([]);
  resolving = $state(false);
  version = $state(0);
  locked = $state<{ url: string; kind: CloudLockKind }[]>([]);
  folders = new Map<string, CloudFolderResult>(folderCache);
  #unlocked: Attachment[] = [];
  #lastKey = '';
  #run = 0;

  cached(url: string): CloudFolderResult | undefined {
    return this.folders.get(url) ?? folderCache.get(url);
  }

  isLocked(url: string): boolean {
    return this.locked.some((link) => link.url === url);
  }

  markLocked(url: string, kind: CloudLockKind) {
    this.locked = [...this.locked.filter((link) => link.url !== url), { url, kind }];
  }

  async fetch(url: string, context?: CloudLinkContext): Promise<CloudFolderResult> {
    const result = await apiResolveCloudLink(url, context);
    this.#remember(url, result);
    this.version++;
    return result;
  }

  unlock(url: string, result: CloudFolderResult) {
    this.#remember(url, result);
    if (!this.isLocked(url)) return;
    this.#unlocked = [...this.#unlocked, ...cloudResultAttachments(result)];
    this.resolved = this.#withUnlocked(this.resolved);
    this.version++;
  }

  sync(postKey: string, urls: string[], context?: CloudLinkContext) {
    const key = `${postKey}:${urls.join('|')}`;
    const sameKey = key === this.#lastKey;
    this.#lastKey = key;
    if (!sameKey) {
      this.locked = [];
      this.#unlocked = [];
    }

    const cachedNodes = postNodesCache.get(key);
    if (cachedNodes && cachedNodes.length > 0 && !missingSizes(cachedNodes)) {
      this.#run++;
      this.resolved = cachedNodes;
      this.resolving = false;
      return;
    }

    if (urls.length === 0) {
      this.#run++;
      this.resolved = [];
      this.#dropVariants(postKey, key);
      this.resolving = false;
      return;
    }

    if (sameKey && this.resolved.length > 0 && !missingSizes(this.resolved)) return;

    const run = ++this.#run;
    const isCurrent = () => run === this.#run;
    if (!sameKey) this.resolved = [];
    this.resolving = true;
    void this.#resolveAll(postKey, urls, context, isCurrent);
  }

  async #resolveAll(postKey: string, urls: string[], context: CloudLinkContext | undefined, isCurrent: () => boolean) {
    const byLink: Attachment[][] = urls.map(() => []);
    let current: Attachment[] = [];
    let cursor = 0;
    const worker = async () => {
      while (cursor < urls.length && isCurrent()) {
        const index = cursor++;
        const nodes = await this.#resolveOne(urls[index], context, isCurrent);
        if (nodes.length > 0 && isCurrent()) {
          byLink[index] = nodes;
          current = byLink.flat();
          this.resolved = this.#withUnlocked(current);
          this.version++;
        }
      }
    };
    const workers = Math.min(4, urls.length);
    await Promise.all(Array.from({ length: workers }, worker));
    if (!isCurrent()) return;
    const cacheKey = `${postKey}:${urls.join('|')}`;
    if (current.length > 0 && this.locked.length === 0) {
      this.#dropVariants(postKey, cacheKey);
      postNodesCache.set(cacheKey, current);
    }
    this.resolving = false;
  }

  #dropVariants(postKey: string, keepKey: string) {
    const prefix = `${postKey}:`;
    for (const existing of postNodesCache.keys()) {
      if (existing !== keepKey && existing.startsWith(prefix)) postNodesCache.delete(existing);
    }
  }

  async #resolveOne(url: string, context: CloudLinkContext | undefined, isCurrent: () => boolean): Promise<Attachment[]> {
    try {
      let result = folderCache.get(url);
      if (!result || result.nodes.some((node) => !node.is_folder && (!node.size || node.size === 0))) {
        result = await apiResolveCloudLink(url, context);
        logger.info(`[Cloud] Resolved link: ${url} (${result.nodes.length} items)`);
      }
      this.#remember(url, result);
      return cloudResultAttachments(result);
    } catch (error) {
      const kind = cloudLockKind(error);
      if (kind) {
        if (isCurrent()) this.markLocked(url, kind);
      } else {
        logger.warn(`Failed to auto-resolve cloud link for post gallery: ${url}`, error);
      }
      return [];
    }
  }

  #remember(url: string, result: CloudFolderResult) {
    folderCache.set(url, result);
    this.folders.set(url, result);
  }

  #withUnlocked(resolved: Attachment[]): Attachment[] {
    const extra = this.#unlocked.filter((att) => !resolved.some((existing) => isSameAttachment(existing, att)));
    return extra.length > 0 ? [...resolved, ...extra] : resolved;
  }
}
