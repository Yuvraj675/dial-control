/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        cmf: {
          orange: "#FF5500",
          "orange-hover": "#FF6A00",
          "orange-glow": "rgba(255, 85, 0, 0.35)",
          cyan: "#00E5FF",
          "cyan-glow": "rgba(0, 229, 255, 0.35)",
          red: "#D71921",
        },
        obsidian: {
          950: "#08090A",
          900: "#0B0C0E",
          850: "#101216",
          800: "#14161B",
          700: "#1E2129",
          border: "rgba(255, 255, 255, 0.08)",
          "border-bright": "rgba(255, 255, 255, 0.18)",
        }
      },
      fontFamily: {
        mono: ["'Space Mono'", "'JetBrains Mono'", "monospace"],
        sans: ["'Inter'", "system-ui", "sans-serif"],
      },
      boxShadow: {
        "cmf-glow": "0 0 24px rgba(255, 85, 0, 0.45)",
        "cyan-glow": "0 0 24px rgba(0, 229, 255, 0.45)",
        "hud-card": "0 12px 40px rgba(0, 0, 0, 0.65)",
      }
    },
  },
  plugins: [],
}
