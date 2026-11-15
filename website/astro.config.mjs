import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const mod = (name) => path.join(here, 'node_modules', name);

// Defaults serve the GitHub Pages project site. For a custom domain set SITE_URL=https://example.com SITE_BASE=/
const site = process.env.SITE_URL || 'https://yuvraj675.github.io';
const base = process.env.SITE_BASE ?? '/dial-control';

export default defineConfig({
  site,
  base,
  trailingSlash: 'always',
  integrations: [react(), sitemap()],
  build: { inlineStylesheets: 'auto' },
  vite: {
    resolve: {
      // The wheel is the app's own component (../src), so it must share this project's React and Motion.
      dedupe: ['react', 'react-dom', 'framer-motion', 'lucide-react'],
      alias: {
        react: mod('react'),
        'react-dom': mod('react-dom'),
        'framer-motion': mod('framer-motion'),
        'lucide-react': mod('lucide-react'),
      },
    },
    server: { fs: { allow: ['..'] } },
  },
});
