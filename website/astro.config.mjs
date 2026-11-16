import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';

// Netlify provides URL (the site's address, or the custom domain once attached) at build time. Set SITE_URL and
// SITE_BASE to build for anywhere else (a subpath host needs SITE_BASE=/name). `||`, not `??`, so empty values fall back.
const site = process.env.SITE_URL || process.env.URL || 'https://dial-control.netlify.app';
const base = process.env.SITE_BASE || '/';

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
