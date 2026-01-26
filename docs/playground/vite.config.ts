import { defineConfig } from 'vite';
import preact from '@preact/preset-vite';

export default defineConfig({
  plugins: [preact()],
  build: {
    rollupOptions: {
      input: {
        main: new URL('./index.html', import.meta.url).pathname,
        embed: new URL('./embed.html', import.meta.url).pathname,
      },
    },
  },
  server: {
    fs: {
      // Allow serving files from the WASM directory
      allow: ['..', './src/wasm'],
    },
  },
});