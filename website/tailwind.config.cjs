/** Utilities only, for the app's wheel component. The page itself is hand-written CSS on tokens. */
module.exports = {
  content: ['./src/**/*.{astro,tsx}', '../src/components/hud/**/*.tsx', '../src/utils/iconMap.tsx'],
  corePlugins: { preflight: false },
  theme: { extend: {} },
  plugins: [],
};
