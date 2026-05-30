/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,svelte,ts}"],
  theme: {
    extend: {
      colors: {
        "shed-black": "#1a1a1a",
        "shed-orange": "#ea580c",
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "ui-monospace", "monospace"],
      },
    },
  },
  plugins: [],
};
