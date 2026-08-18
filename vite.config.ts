import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import Icons from 'unplugin-icons/vite';
import path from 'path';
import { execSync } from 'child_process';

let commitHash = 'unknown';
try {
  commitHash = execSync('git rev-parse --short HEAD').toString().trim();
} catch {}

const buildTime = new Date().toISOString();

// @ts-ignore
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  define: {
    __BUILD_TIME__: JSON.stringify(buildTime),
    __COMMIT_HASH__: JSON.stringify(commitHash)
  },
  plugins: [
    tailwindcss(),
    svelte(),
    Icons({
      compiler: 'svelte',
      autoInstall: true
    })
  ],
  resolve: {
    alias: {
      '$lib': path.resolve(__dirname, './src/lib')
    }
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  clearScreen: false,
});
