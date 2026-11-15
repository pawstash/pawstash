const AVATAR_SIZE = 256;
const AVATAR_MAX_BYTES = 28 * 1024;
const INPUT_MAX_BYTES = 25 * 1024 * 1024;
const DECODE_MAX_EDGE = 1024;

function decodedLength(dataUrl: string): number {
  const base64 = dataUrl.slice(dataUrl.indexOf(',') + 1);
  const padding = base64.endsWith('==') ? 2 : base64.endsWith('=') ? 1 : 0;
  return Math.floor((base64.length * 3) / 4) - padding;
}

async function decodeForAvatar(file: File): Promise<ImageBitmap | HTMLImageElement> {
  if (typeof createImageBitmap === 'function') {
    try {
      return await createImageBitmap(
        file,
        file.size > 2 * 1024 * 1024
          ? { resizeWidth: DECODE_MAX_EDGE, resizeQuality: 'high' }
          : undefined
      );
    } catch {
    }
  }
  const objectUrl = URL.createObjectURL(file);
  try {
    const image = new Image();
    image.src = objectUrl;
    await image.decode();
    return image;
  } finally {
    URL.revokeObjectURL(objectUrl);
  }
}

export async function fileToAvatarDataUrl(file: File): Promise<string> {
  if (!file.type.startsWith('image/')) {
    throw new Error('Not an image file');
  }
  if (file.size > INPUT_MAX_BYTES) {
    throw new Error('Image is too large');
  }
  const source = await decodeForAvatar(file);
  try {
    const sourceWidth = source instanceof HTMLImageElement ? source.naturalWidth : source.width;
    const sourceHeight = source instanceof HTMLImageElement ? source.naturalHeight : source.height;
    if (!sourceWidth || !sourceHeight) {
      throw new Error('Image has no pixels');
    }
    const canvas = document.createElement('canvas');
    canvas.width = AVATAR_SIZE;
    canvas.height = AVATAR_SIZE;
    const context = canvas.getContext('2d');
    if (!context) {
      throw new Error('Canvas is unavailable');
    }
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = 'high';
    const scale = Math.max(AVATAR_SIZE / sourceWidth, AVATAR_SIZE / sourceHeight);
    const width = Math.round(sourceWidth * scale);
    const height = Math.round(sourceHeight * scale);
    context.drawImage(
      source,
      Math.round((AVATAR_SIZE - width) / 2),
      Math.round((AVATAR_SIZE - height) / 2),
      width,
      height
    );
    for (const quality of [0.86, 0.76, 0.66, 0.56]) {
      const dataUrl = canvas.toDataURL('image/jpeg', quality);
      if (decodedLength(dataUrl) <= AVATAR_MAX_BYTES) {
        return dataUrl;
      }
    }
    return canvas.toDataURL('image/jpeg', 0.45);
  } finally {
    if (!(source instanceof HTMLImageElement)) {
      source.close();
    }
  }
}
