import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';

// Defaults serve the GitHub Pages project site. For a custom domain set SITE_URL=https://example.com SITE_BASE=/
// `||`, not `??`: the workflow passes unset repository variables as empty strings.
const site = process.env.SITE_URL || 'https://yuvraj675.github.io';
const base = process.env.SITE_BASE || '/dial-control';

export default defineConfig({
  site,
  base,
  trailingSlash: 'always',
  integrations: [react(), sitemap()],
  build: { inlineStylesheets: 'auto' },
  vite: {
    resolve: {
      // The wheel is the app's own component (../src); dedupe makes it share this project's single
      // copy of React and Motion instead of the repo root's (two copies break hooks).
      dedupe: ['react', 'react-dom', 'framer-motion', 'lucide-react'],
    },
    server: { fs: { allow: ['..'] } },
  },
});
