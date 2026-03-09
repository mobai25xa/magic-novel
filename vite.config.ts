import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

function manualChunks(id: string) {
  const normalized = id.replace(/\\/g, '/')

  if (normalized.includes('/node_modules/lucide-react/')) {
    return 'vendor-lucide'
  }

  if (
    normalized.includes('/node_modules/@tiptap/')
    || normalized.includes('/node_modules/prosemirror-')
  ) {
    return 'vendor-tiptap'
  }

  return undefined
}

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    chunkSizeWarningLimit: 550,
    rollupOptions: {
      output: {
        manualChunks,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
