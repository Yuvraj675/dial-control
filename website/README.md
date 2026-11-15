# Dial Control website

The marketing site for Dial Control, built with [Astro](https://astro.build), React islands and Framer Motion. It is static HTML (fast, crawlable) with two interactive islands that run **the app's own wheel component** (`../src/components/hud/DonutRadialHUD.tsx`) from the app's own profile data, so the page always shows the real product.

```bash
cd website
npm ci
npm run dev        # http://localhost:4321/dial-control/
npm run build      # static site in dist/
npm run preview
```

## How it is put together

| Path | What it is |
|---|---|
| `src/pages/index.astro` | The whole page, the SEO head (title, description, Open Graph, JSON-LD) and the OS-aware download script |
| `src/components/HeroWheel.tsx` | Hero island: simulated double press, then the live wheel (arrow keys, Enter, Escape, buttons, clicks) |
| `src/components/ProfileSwitcher.tsx` | The six built-in wheels, switchable |
| `src/styles/tokens.css`, `base.css`, `global.css` | Design tokens, base rules and the page styles (OKLCH tokens, Doto and Hanken Grotesk) |
| `src/styles/wheel.css` | Tailwind utilities for the app component only (no preflight) |
| `public/images`, `src/assets/fonts` | Captured product images and icons, and the self-hosted OFL fonts (imported from CSS so URLs include the site base; licences are in `public/fonts`) |

React and Motion are shared with the app's component through `resolve.dedupe` in `astro.config.mjs`. Do not replace that with an `alias` to `node_modules/react`: it builds, but the dev server then fails with `module is not defined`.

The app component gets a small `inline` prop so it renders in normal flow instead of as a full-screen overlay. Keep that prop working when you change the component.

## Downloads

The page never hard-codes a version. In the browser it detects the visitor's OS and asks the GitHub API for the latest release, then points each button at the matching asset (`*_x64-setup.exe`, `*_aarch64.dmg`, `.AppImage`, `.deb`, `.rpm`, `.pkg.tar.zst`). Until a release exists the buttons link to the Releases page. The asset names come from `.github/workflows/release.yml`, so do not rename them there without updating the patterns in `index.astro`.

## Deploying

`.github/workflows/pages.yml` builds and publishes to GitHub Pages on changes to `website/` or the embedded app code, when a release is published, and on demand.

One-time setup in the repository: **Settings → Pages → Source: GitHub Actions**. The site is then at `https://<owner>.github.io/dial-control/`.

### Custom domain

1. Buy a domain and add it under **Settings → Pages → Custom domain** (GitHub shows the DNS records to create).
2. Add repository **variables** (Settings → Secrets and variables → Actions → Variables): `SITE_URL=https://yourdomain.com` and `SITE_BASE=/`.
3. Re-run the workflow. Canonical URLs, Open Graph tags, the sitemap and `robots.txt` all follow those two values.

## SEO checklist

Already in place: one `h1` and a clean heading outline, a 57-character title and 149-character description, canonical URL, Open Graph and Twitter cards with a 1280x640 image, JSON-LD (`SoftwareApplication`, `FAQPage`, `WebSite`), sitemap and generated `robots.txt`, alt text on every image, preloaded fonts and LCP image, no layout shift from the hero, and the full content visible without JavaScript.

After the first deploy:
1. Verify the site in [Google Search Console](https://search.google.com/search-console) and submit `sitemap-index.xml`.
2. Set the repository's **Website** field (About panel) to the site URL so GitHub links to it.
3. Link to the site from the README, the release notes and anywhere the project is mentioned. Links from other sites are what move rankings most.
4. Test with PageSpeed Insights and the Rich Results Test.

## Rules for changing the page

- Only real facts: no invented numbers, testimonials or logos. Claims must match the README.
- No third-party branding, and keep the "not affiliated with Nothing" notice.
- Images of the product are captures of the real app (see git history of `public/images` for the capture approach), never drawn mock-ups.
- Keep each screen under about 80 words and run Hallmark's `sloplint` if you use that skill.

Fonts: [Doto](https://fonts.google.com/specimen/Doto) and [Hanken Grotesk](https://fonts.google.com/specimen/Hanken+Grotesk), SIL Open Font License (licence files in `public/fonts`).
