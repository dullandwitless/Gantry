import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import fs from 'fs';
import path from 'path';

// Plugin to ensure 200.html exists for Axum SPA fallback
function copyIndexTo200() {
  return {
    name: 'copy-index-to-200',
    closeBundle() {
      const outDir = path.resolve(__dirname, 'build');
      const indexPath = path.join(outDir, 'index.html');
      const fallbackPath = path.join(outDir, '200.html');
      if (fs.existsSync(indexPath)) {
        fs.copyFileSync(indexPath, fallbackPath);
        console.log('[vite] Generated 200.html from index.html for Axum static_assets fallback.');
      }
    }
  };
}

export default defineConfig({
  plugins: [react(), copyIndexTo200()],
  build: {
    outDir: 'build',
    emptyOutDir: true,
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8000',
        changeOrigin: true,
      },
      '/w': {
        target: 'http://localhost:8000',
        changeOrigin: true,
      },
    },
  },
});
