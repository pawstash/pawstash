import { toast, type ExternalToast } from 'svelte-sonner';

export type ToastOptions = ExternalToast;

function parseErrorDescription(error: unknown): string | undefined {
  if (!error) return undefined;
  if (typeof error === 'string') {
    const trimmed = error.trim();
    return trimmed.length > 0 ? trimmed : undefined;
  }
  if (error instanceof Error) {
    return error.message.trim() || undefined;
  }
  if (typeof error === 'object' && error !== null && 'message' in error && typeof (error as any).message === 'string') {
    return (error as any).message.trim() || undefined;
  }
  try {
    const stringified = String(error).trim();
    return stringified !== '[object Object]' ? stringified : undefined;
  } catch {
    return undefined;
  }
}

export const notify = {
  success(title: string, descriptionOrOptions?: string | ToastOptions, extraOptions?: ToastOptions) {
    if (typeof descriptionOrOptions === 'object' && descriptionOrOptions !== null) {
      return toast.success(title, descriptionOrOptions);
    }
    const description = descriptionOrOptions ? String(descriptionOrOptions).trim() : undefined;
    return toast.success(title, {
      description: description || undefined,
      ...extraOptions,
    });
  },

  error(title: string, errorOrDescription?: unknown, extraOptions?: ToastOptions) {
    if (typeof errorOrDescription === 'object' && errorOrDescription !== null && !('message' in errorOrDescription) && !Array.isArray(errorOrDescription)) {
      // It might be a direct ToastOptions object
      return toast.error(title, errorOrDescription as ToastOptions);
    }
    const description = parseErrorDescription(errorOrDescription);
    return toast.error(title, {
      description,
      ...extraOptions,
    });
  },

  info(title: string, descriptionOrOptions?: string | ToastOptions, extraOptions?: ToastOptions) {
    if (typeof descriptionOrOptions === 'object' && descriptionOrOptions !== null) {
      return toast.info(title, descriptionOrOptions);
    }
    const description = descriptionOrOptions ? String(descriptionOrOptions).trim() : undefined;
    return toast.info(title, {
      description: description || undefined,
      ...extraOptions,
    });
  },

  warning(title: string, descriptionOrOptions?: string | ToastOptions, extraOptions?: ToastOptions) {
    if (typeof descriptionOrOptions === 'object' && descriptionOrOptions !== null) {
      return toast.warning(title, descriptionOrOptions);
    }
    const description = descriptionOrOptions ? String(descriptionOrOptions).trim() : undefined;
    return toast.warning(title, {
      description: description || undefined,
      ...extraOptions,
    });
  },

  dismiss(toastId?: number | string) {
    return toast.dismiss(toastId);
  },

  loading(title: string, options?: ToastOptions) {
    return toast.loading(title, options);
  },
};

export { toast };
