import type { APIRoute } from 'astro';

// Built from the configured site URL, so a custom domain needs no edit here.
export const GET: APIRoute = ({ site }) => {
  const base = import.meta.env.BASE_URL.replace(/\/$/, '');
  const origin = (site ?? new URL('https://dial-control.netlify.app')).toString().replace(/\/$/, '');
  return new Response(`User-agent: *\nAllow: /\n\nSitemap: ${origin}${base}/sitemap-index.xml\n`, {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
};
