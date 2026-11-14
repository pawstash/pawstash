import { mount } from 'svelte';
import './app.css';
import { i18n } from './lib/i18n';
import { dismissBootScreen, syncBootTheme } from './lib/utils/bootScreen';
import App from './routes/+page.svelte';

const target = document.getElementById('app')!;

syncBootTheme();

async function boot() {
  await i18n.init().catch(() => {});
  mount(App, { target });
}

void boot();
window.setTimeout(dismissBootScreen, 8000);
