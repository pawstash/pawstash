const AVATAR_SIZE = 256;
const AVATAR_MAX_BYTES = 28 * 1024;

function decodedLength(dataUrl: string): number {
  const base64 = dataUrl.slice(dataUrl.indexOf(',') + 1);
  const padding = base64.endsWith('==') ? 2 : base64.endsWith('=') ? 1 : 0;
  return Math.floor((base64.length * 3) / 4) - padding;
}

export async function fileToAvatarDataUrl(file: File): Promise<string> {
  if (!file.type.startsWith('image/')) {
    throw new Error('Not an image file');
  }
  const objectUrl = URL.createObjectURL(file);
  try {
    const image = new Image();
    image.src = objectUrl;
    await image.decode();
    if (!image.naturalWidth || !image.naturalHeight) {
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
    const scale = Math.max(AVATAR_SIZE / image.naturalWidth, AVATAR_SIZE / image.naturalHeight);
    const width = Math.round(image.naturalWidth * scale);
    const height = Math.round(image.naturalHeight * scale);
    context.drawImage(
      image,
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
    URL.revokeObjectURL(objectUrl);
  }
}
