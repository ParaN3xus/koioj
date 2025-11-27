/** @type {import('tailwindcss').Config} */

import typography from "@tailwindcss/typography";
import daisyui from "daisyui";

export default {
  content: [
    "./crates/koioj-web/index.html",
    "./crates/koioj-web/src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [daisyui, typography],
  daisyui: {
    themes: ["light", "dark"],
  },
};
