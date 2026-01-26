import { defineConfig } from 'vite';
import { fileURLToPath } from 'url';
import preact from '@preact/preset-vite';

export default defineConfig({
  plugins: [preact()],
  build: {
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        embed: fileURLToPath(new URL('./embed.html', import.meta.url)),
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