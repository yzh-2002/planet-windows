import { resolve } from 'path'
import { defineConfig } from 'electron-vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  main: {
    resolve: {
      alias: {
        '@main': resolve('src/main'),
        '@ipfs': resolve('src/main/ipfs'),
        '@data': resolve('src/main/data'),
        '@services': resolve('src/main/services'),
      }
    }
  },
  preload: {},
  renderer: {
    resolve: {
      alias: {
        '@renderer': resolve('src/renderer/src'),
        '@store': resolve('src/renderer/src/store'),
        '@pages': resolve('src/renderer/src/pages'),
        '@components': resolve('src/renderer/src/components'),
      }
    },
    plugins: [react(), tailwindcss()]
  }
})
