/** @type {import('tailwindcss').Config} */

import typography from "@tailwindcss/typography";
import daisyui from "daisyui";

export default {
  content: [
    "./crates/theoj-web/index.html",
    "./crates/theoj-web/src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [daisyui, typography],
  daisyui: {
    themes: ["light", "dark"],
  },
};
