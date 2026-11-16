import type { APIRoute } from 'astro';
import { askAiPrompt } from '../content/askAi';

// The llms.txt convention: a plain Markdown summary at the site root for language models. It is the same
// text the "Ask your AI" badge copies, so the two never drift apart.
export const GET: APIRoute = ({ site }) => {
  const base = import.meta.env.BASE_URL;
  const origin = (site ?? new URL('https://dial-control.netlify.app')).toString().replace(/\/$/, '');
  const url = `${origin}${base.endsWith('/') ? base : base + '/'}`;
  return new Response(askAiPrompt(url, 'https://github.com/Yuvraj675/dial-control'), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
};
