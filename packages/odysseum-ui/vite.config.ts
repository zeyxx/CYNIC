import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    nodePolyfills({
      include: ['buffer', 'crypto', 'stream', 'util'],
      globals: { Buffer: true, global: true, process: true },
    }),
  ],
  server: {
    host: '0.0.0.0', // Required for Docker
    port: 5173,
    proxy: {
      '/api/cynic': {
        target: process.env.CYNIC_REST_ADDR || 'http://10.10.10.140:3030',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api\/cynic/, ''),
      },
    },
  },
})
