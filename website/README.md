# Dial Control website

The marketing site for Dial Control, built with [Astro](https://astro.build), React islands and Framer Motion. It is static HTML (fast, crawlable) with two interactive islands that run **the app's own wheel component** (`../src/components/hud/DonutRadialHUD.tsx`) from the app's own profile data, so the page always shows the real product.

```bash
cd website
npm ci
npm run dev        # http://localhost:4321/
npm run build      # static site in dist/
npm run preview
```

## How it is put together

| Path | What it is |
|---|---|
| `src/pages/index.astro` | The whole page, the SEO head (title, description, Open Graph, JSON-LD) and the OS-aware download script |
| `src/components/FlyingDial.tsx` | The one live wheel. It sits in the hero, flies out in 3D on scroll, pins in front of "How the dial works" and performs each step |
| `src/components/diagramData.ts` | The walkthrough: each label, where its line attaches, and `dialAt(q)`, which says what the dial shows at any scroll progress (tested) |
| `src/scripts/diagram.ts` | anime.js: lays out the labels around the pinned dial and draws their leader lines, synced to scroll |
| `src/scripts/annotate.ts` | anime.js: the Settings window tilts in 3D to flat, then its labelled lines draw |
| `src/scripts/keys.ts` | anime.js: the keyboard section's keys fall flat in 3D |
| `src/components/ProfileSwitcher.tsx` | The six built-in wheels; the dial leans toward the pointer |
| `src/styles/tokens.css`, `base.css`, `global.css` | Design tokens, base rules and the page styles (OKLCH tokens, Doto and Hanken Grotesk) |
| `src/styles/wheel.css` | Tailwind utilities for the app component only (no preflight) |
| `public/images`, `src/assets/fonts` | Captured product images and icons, and the self-hosted OFL fonts (imported from CSS so URLs include the site base; licences are in `public/fonts`) |

The app component gets a small `inline` prop so it renders in normal flow instead of as a full-screen overlay. Keep that prop working when you change the component.

React and Motion are shared with the app's component through `resolve.dedupe` in `astro.config.mjs`. Do not replace that with an `alias` to `node_modules/react`: it builds, but the dev server then fails with `module is not defined`.

## The 3D scroll effects

3D and 2D are meant to work together: a real 3D-tilted object (the dial, the Settings window, the keys) with crisp 2D labels and lines drawn to it.

- **The dial's flight.** The dial is positioned in three modes that hand over at exact matches, so there is never a jump: absolute in the page while it flies (it scrolls natively, and the script only adds the tilt, push and scale on a bell curve), `position: fixed` while the diagram is pinned (no script movement at all), and absolute again at the spot where it was pinned once the diagram lets go. `FlyingDial.tsx` measures the hero placeholder and the diagram placeholder, so nothing is hard-coded.
- **The walkthrough.** `CALLOUTS` in `diagramData.ts` is the single source for the labels in the HTML, the line anchors and the timing. `dialAt(q)` is a pure function of scroll progress; change it and its test together. Each step is `STEP_LENGTH` of the walkthrough and has two halves: the label and its line draw in the first half, and the dial only acts in the second, so the words are always there before anything happens (tested). The section is 900svh tall, about one screen of scrolling per step; anime.js starts its scroll range at the pin offset (`container: 72`) so its timeline and `dialAt` stay aligned.
- **Where it runs.** Only at 72rem and wider with motion allowed (the `js-3d` class, set by a tiny inline script in the head and kept in sync by `FlyingDial`). Phones, narrow windows, reduced motion and no-JavaScript get the finished page: the dial stays in the hero and the steps are a plain list. Never hide content at rest behind `js-3d` without a script path that removes the class on failure.
- **Clean leader lines.** Following the usual rules for annotated diagrams (leaders never cross, are horizontal or one fixed angle, are parallel where possible and thinner than the subject), `layoutLeaders()` gives every line a 45 degree run out of its anchor and a level run into its label, anchors sit just inside the outer rim of their option, and labels are ordered like their anchors. Finished lines rest in grey and the current step is red. Tests check, for six dial sizes, that no two lines cross, that every segment is level or at 45 degrees, that no line runs through another anchor and that labels do not overlap; change the label slots in `CALLOUTS` and the tests will say if a layout turns messy.
- **Tests.** `npm test` covers the choreography and the line layout.

## Small interactions

`src/scripts/interact.ts` holds the pointer and keyboard touches. They switch themselves off on touch screens and under reduced motion, and none of them changes layout:

- **Word-by-word headings and dot scramble.** Section headings below the first screen rise in word by word the first time they scroll into view (the script adds the classes, so with scripting off or under reduced motion they just read normally). The wordmark and platform names re-draw their letters through dots once per hover or focus, fine pointer only.
- **Scroll snap.** Each section settles at the top under the nav (`scroll-snap-type: y proximity`, never mandatory, so the pinned walkthrough and tall sections still scroll freely). Off for reduced motion and touch screens.
- **Ask your AI.** A centred 3D pill under the install steps (it tilts up to 12 degrees toward the pointer with a sheen that follows it, only when the pointer is on the pill itself, and plays the same wiggle, sweep and action-line invitation as the navbar button, offset in time) copies `src/content/ask-ai.md` (placeholders `{{SITE}}` and `{{REPO}}` filled at build) to the clipboard; the same text is served at `/llms.txt`. `src/content/ask-ai.test.ts` guards its facts, links and length. Update the prompt whenever a user-visible feature or requirement changes.
- **Tilt.** Any element with `data-tilt="<degrees>"` leans toward the pointer (the Settings window; the app switcher and the dial do the same through Framer Motion).
- **Live keys.** The keys in the keyboard section light up while you hold the real ones (`data-key` holds the `KeyboardEvent.key` value).

To test pointer effects in headless Chrome, launch it with `--blink-settings=primaryPointerType=4,primaryHoverType=2,availablePointerTypes=4,availableHoverTypes=2` (headless reports no pointer), and scroll with `behavior: 'instant'` because the page uses smooth scrolling.

## Downloads

The page never hard-codes a version. In the browser it detects the visitor's OS and asks the GitHub API for the latest release, then points each button at the matching asset (`*_x64-setup.exe`, `*_aarch64.dmg`, `.AppImage`, `.deb`, `.rpm`, `.pkg.tar.zst`). Until a release exists the buttons link to the Releases page. The asset names come from `.github/workflows/release.yml`, so do not rename them there without updating the patterns in `index.astro`.

## Deploying

The site is hosted on Netlify, configured by `netlify.toml` at the repository root: base directory `website`, command `npm test && npm run build`, publish `dist`, Node 22, long-lived caching for fingerprinted assets and a few security headers. Every push to `main` deploys; other branches and pull requests get deploy previews. Netlify supplies the `URL` variable at build time, which Astro uses for canonical links, Open Graph tags, the sitemap, `robots.txt` and `/llms.txt`.

One-time setup: **Netlify → Add new site → Import an existing project → GitHub → Yuvraj675/dial-control**. Netlify reads `netlify.toml`, so leave the build settings as detected. Then **Site configuration → Change site name** to `dial-control` (the address in `astro.config.mjs` and the README assumes `https://dial-control.netlify.app`; if the name is taken, set the repository's `SITE_URL` build variable in Netlify to the real address).

### Custom domain (optional)

Add it under **Domain management** in Netlify. `URL` then becomes the custom domain automatically and the next deploy follows it. Building for a subpath host instead needs `SITE_URL` and `SITE_BASE=/name`.

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
