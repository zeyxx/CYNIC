import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  cacheDir: '/tmp/cynic-ui-vite-cache',
  define: {
    'process.env': {},
    global: 'globalThis',
  },
  server: {
    host: '0.0.0.0',
    port: 5000,
    strictPort: true,
    allowedHosts: true,
    proxy: {
      '/api': {
        target: 'http://localhost:3030',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ''),
      },
      '/organ-discord': {
        target: 'http://localhost:8891',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/organ-discord/, ''),
      },
    },
  },
})
