import { invoke } from '@tauri-apps/api/core';

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

class Logger {
  private formatMessage(level: LogLevel, message: string, context?: unknown): string {
    const timestamp = new Date().toISOString();
    return `[${timestamp}] [${level.toUpperCase()}] ${message}`;
  }

  debug(message: string, context?: unknown) {
    if (import.meta.env.DEV) {
      console.debug(`%c[DEBUG]%c ${message}`, 'color: #9ca3af; font-weight: bold;', 'color: inherit;', context ?? '');
    }
    this.send('debug', message, context);
  }

  info(message: string, context?: unknown) {
    console.info(`%c[INFO]%c ${message}`, 'color: #38bdf8; font-weight: bold;', 'color: inherit;', context ?? '');
    this.send('info', message, context);
  }

  warn(message: string, context?: unknown) {
    console.warn(`%c[WARN]%c ${message}`, 'color: #facc15; font-weight: bold;', 'color: inherit;', context ?? '');
    this.send('warn', message, context);
  }

  error(message: string, error?: unknown, context?: unknown) {
    let errorDetails = '';
    if (error instanceof Error) {
      errorDetails = ` — ${error.message}\n${error.stack ?? ''}`;
    } else if (typeof error === 'string') {
      errorDetails = ` — ${error}`;
    } else if (error) {
      try {
        errorDetails = ` — ${JSON.stringify(error)}`;
      } catch {
        errorDetails = ` — ${String(error)}`;
      }
    }

    const fullMessage = `${message}${errorDetails}`;
    console.error(`%c[ERROR]%c ${fullMessage}`, 'color: #f87171; font-weight: bold;', 'color: inherit;', context ?? '');
    this.send('error', fullMessage, context);
  }

  private send(level: LogLevel, message: string, context?: unknown) {
    let serializedContext: unknown = undefined;
    if (context !== undefined) {
      try {
        serializedContext = JSON.parse(JSON.stringify(context));
      } catch {
        serializedContext = String(context);
      }
    }

    invoke('log_message', {
      level,
      message,
      context: serializedContext
    }).catch(() => {});
  }
}

export const logger = new Logger();

export function logMediaError(kind: 'video' | 'audio' | 'image', src: string, name?: string, error?: MediaError | Event | null) {
  let detail = '';
  if (error && typeof error === 'object' && 'code' in error) {
    const me = error as MediaError;
    const codeName = me.code === 1 ? 'MEDIA_ERR_ABORTED' :
      me.code === 2 ? 'MEDIA_ERR_NETWORK' :
      me.code === 3 ? 'MEDIA_ERR_DECODE' :
      me.code === 4 ? 'MEDIA_ERR_SRC_NOT_SUPPORTED' : `UNKNOWN_CODE_${me.code}`;
    detail = ` [code: ${me.code} (${codeName}), message: "${me.message}"]`;
  }
  logger.error(`Media element error: <${kind}> failed to load "${name || 'unnamed'}"${detail} | URL: ${src}`);
}

export async function getDebugLogPath(): Promise<string> {
  return invoke<string>('get_debug_log_path');
}

export async function readRecentLogs(lines = 500): Promise<string> {
  return invoke<string>('read_recent_logs', { lines });
}

export async function openLogsFolder(): Promise<void> {
  return invoke<void>('open_logs_folder');
}

export async function clearLogs(): Promise<void> {
  return invoke<void>('clear_logs');
}

let initialized = false;
export function initFrontendLogging() {
  if (initialized || typeof window === 'undefined') return;
  initialized = true;

  window.addEventListener('error', (event) => {
    logger.error(`Unhandled exception: ${event.message} at ${event.filename}:${event.lineno}:${event.colno}`, event.error);
  });

  window.addEventListener('unhandledrejection', (event) => {
    logger.error('Unhandled Promise rejection', event.reason);
  });

  logger.info('Frontend logging and global exception handlers initialized');
}
