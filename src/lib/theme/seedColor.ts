import {
  Hct,
  QuantizerCelebi,
  Score,
  argbFromRgb,
  hexFromArgb
} from '@material/material-color-utilities';
import { thumbHashToRGBA } from 'thumbhash';
import { logger } from '$lib/utils/logger';

const SAMPLE_SIZE = 64;
const MAX_CLUSTERS = 128;
const MIN_SEED_CHROMA = 10;
const NO_SEED_SENTINEL = 0xff000001;

const thumbHashSeeds = new Map<string, string | null>();
const urlSeeds = new Map<string, string | null>();

function seedFromRgba(rgba: Uint8Array | Uint8ClampedArray): string | null {
  const pixels: number[] = [];
  for (let index = 0; index < rgba.length; index += 4) {
    if (rgba[index + 3] < 200) continue;
    pixels.push(argbFromRgb(rgba[index], rgba[index + 1], rgba[index + 2]));
  }
  if (!pixels.length) return null;

  const ranked = Score.score(QuantizerCelebi.quantize(pixels, MAX_CLUSTERS), {
    desired: 1,
    fallbackColorARGB: NO_SEED_SENTINEL
  });

  const top = ranked[0];
  if (top === undefined || top === NO_SEED_SENTINEL) return null;
  return Hct.fromInt(top).chroma < MIN_SEED_CHROMA ? null : hexFromArgb(top);
}

export function seedFromThumbHash(base64Hash?: string | null): string | null {
  if (!base64Hash || typeof base64Hash !== 'string') return null;
  if (thumbHashSeeds.has(base64Hash)) return thumbHashSeeds.get(base64Hash) ?? null;

  let seed: string | null = null;
  try {
    const binary = Uint8Array.from(atob(base64Hash), (char) => char.charCodeAt(0));
    const { rgba } = thumbHashToRGBA(binary);
    seed = seedFromRgba(rgba);
  } catch (error) {
    logger.warn('[Seed] thumbhash seed extraction failed', error);
  }

  thumbHashSeeds.set(base64Hash, seed);
  return seed;
}

export function seedFromImageUrl(url: string): Promise<string | null> {
  if (!url) return Promise.resolve(null);
  if (urlSeeds.has(url)) return Promise.resolve(urlSeeds.get(url) ?? null);

  return new Promise((resolve) => {
    const image = new Image();
    image.crossOrigin = 'Anonymous';

    const finish = (seed: string | null) => {
      urlSeeds.set(url, seed);
      resolve(seed);
    };

    image.onload = () => {
      try {
        const canvas = document.createElement('canvas');
        canvas.width = SAMPLE_SIZE;
        canvas.height = SAMPLE_SIZE;
        const context = canvas.getContext('2d', { willReadFrequently: true });
        if (!context) return finish(null);
        context.drawImage(image, 0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
        finish(seedFromRgba(context.getImageData(0, 0, SAMPLE_SIZE, SAMPLE_SIZE).data));
      } catch (error) {
        logger.warn('[Seed] canvas seed extraction failed', error);
        finish(null);
      }
    };
    image.onerror = () => finish(null);
    image.src = url;
  });
}
