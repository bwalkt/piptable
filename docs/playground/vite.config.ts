import { defineConfig } from 'vite';

export default defineConfig({
  base: '/playground/',
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: {
          codemirror: ['codemirror', '@codemirror/commands', '@codemirror/language', '@codemirror/state', '@codemirror/view']
        }
      }
    }
  },
  server: {
    port: 3000,
    open: true
  }
});