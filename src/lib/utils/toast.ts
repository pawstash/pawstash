import { toast, type ExternalToast } from 'svelte-sonner';
import ToastCard, { type ToastGlyph } from '$lib/components/ui/Toast.svelte';

export type ToastVariant = 'success' | 'error' | 'warning' | 'info' | 'loading';

export interface ToastAction {
  label: string;
  onclick: () => void;
}

export type ToastOptions = Omit<ExternalToast, 'action' | 'description' | 'componentProps'> & {
  description?: string;
  action?: ToastAction;
  dismissible?: boolean;
  swatch?: string;
  glyph?: ToastGlyph;
};

const DEFAULT_DURATION: Record<ToastVariant, number> = {
  success: 4000,
  info: 4000,
  warning: 5500,
  error: 7000,
  loading: Number.POSITIVE_INFINITY
};

function parseErrorDescription(error: unknown): string | undefined {
  if (!error) return undefined;
  if (typeof error === 'string') {
    const trimmed = error.trim();
    return trimmed.length > 0 ? trimmed : undefined;
  }
  if (error instanceof Error) {
    return error.message.trim() || undefined;
  }
  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof (error as { message: unknown }).message === 'string'
  ) {
    return (error as { message: string }).message.trim() || undefined;
  }
  try {
    const stringified = String(error).trim();
    return stringified !== '[object Object]' ? stringified : undefined;
  } catch {
    return undefined;
  }
}

interface LiveToast {
  id: string;
  repeat: number;
  expiresAt: number;
}

const live = new Map<string, LiveToast>();

function dedupeKey(variant: ToastVariant, title: string, description?: string) {
  return `${variant}\0${title}\0${description ?? ''}`;
}

function show(variant: ToastVariant, title: string, options: ToastOptions = {}) {
  const { description, action, dismissible = true, swatch, glyph, ...sonnerOptions } = options;
  const desc = description?.trim() || undefined;
  const duration = sonnerOptions.duration ?? DEFAULT_DURATION[variant];
  const now = Date.now();

  for (const [key, entry] of live) {
    if (entry.expiresAt <= now) live.delete(key);
  }

  const key = sonnerOptions.id === undefined ? dedupeKey(variant, title, desc) : null;
  const merged = key ? live.get(key) : undefined;

  const id = sonnerOptions.id
    ?? `pawstash-toast-${now}-${Math.random().toString(36).slice(2, 8)}`;
  const repeat = merged ? merged.repeat + 1 : 1;

  if (key) {
    live.set(key, { id: String(id), repeat, expiresAt: now + duration });
  }

  if (merged) {
    toast.dismiss(merged.id);
  }

  const forget = () => {
    if (key && live.get(key)?.id === String(id)) live.delete(key);
  };

  return toast.custom(ToastCard as never, {
    ...sonnerOptions,
    id,
    duration,
    important: variant === 'error' || sonnerOptions.important,
    onDismiss: forget,
    onAutoClose: forget,
    componentProps: {
      id,
      variant,
      title,
      description: desc,
      swatch,
      glyph,
      actionLabel: action?.label,
      onaction: action?.onclick,
      ondismiss: () => {
        forget();
        toast.dismiss(id);
      },
      dismissible,
      duration,
      repeat
    }
  } as never);
}

function emit(
  variant: ToastVariant,
  title: string,
  descriptionOrOptions?: string | ToastOptions,
  extraOptions?: ToastOptions
) {
  if (typeof descriptionOrOptions === 'object' && descriptionOrOptions !== null) {
    return show(variant, title, { ...descriptionOrOptions, ...extraOptions });
  }
  const description = descriptionOrOptions ? String(descriptionOrOptions).trim() : undefined;
  return show(variant, title, { description: description || undefined, ...extraOptions });
}

export const notify = {
  success(title: string, descriptionOrOptions?: string | ToastOptions, extraOptions?: ToastOptions) {
    return emit('success', title, descriptionOrOptions, extraOptions);
  },

  error(title: string, errorOrDescription?: unknown, extraOptions?: ToastOptions) {
    if (
      typeof errorOrDescription === 'object' &&
      errorOrDescription !== null &&
      !('message' in errorOrDescription) &&
      !Array.isArray(errorOrDescription)
    ) {
      return emit('error', title, errorOrDescription as ToastOptions, extraOptions);
    }
    return show('error', title, {
      description: parseErrorDescription(errorOrDescription),
      ...extraOptions
    });
  },

  info(title: string, descriptionOrOptions?: string | ToastOptions, extraOptions?: ToastOptions) {
    return emit('info', title, descriptionOrOptions, extraOptions);
  },

  warning(title: string, descriptionOrOptions?: string | ToastOptions, extraOptions?: ToastOptions) {
    return emit('warning', title, descriptionOrOptions, extraOptions);
  },

  loading(title: string, options?: ToastOptions) {
    return show('loading', title, options);
  },

  dismiss(toastId?: number | string) {
    return toast.dismiss(toastId);
  },

  async promise<T>(
    work: Promise<T> | (() => Promise<T>),
    messages: {
      loading: string;
      success: string | ((value: T) => string);
      error: string | ((error: unknown) => string);
    }
  ): Promise<T> {
    const id = `pawstash-promise-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    show('loading', messages.loading, { id });
    try {
      const value = await (typeof work === 'function' ? work() : work);
      const title =
        typeof messages.success === 'function' ? messages.success(value) : messages.success;
      show('success', title, { id });
      return value;
    } catch (error) {
      const title = typeof messages.error === 'function' ? messages.error(error) : messages.error;
      show('error', title, { id, description: parseErrorDescription(error) });
      throw error;
    }
  }
};
