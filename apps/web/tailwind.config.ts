import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{js,ts,jsx,tsx,mdx}", "./components/**/*.{js,ts,jsx,tsx,mdx}"],
  theme: {
    extend: {
      keyframes: {
        "toast-in": {
          from: { opacity: "0", transform: "translateX(1rem)" },
          to: { opacity: "1", transform: "translateX(0)" },
        },
        "modal-in": {
          from: { opacity: "0", transform: "scale(0.96) translateY(0.5rem)" },
          to: { opacity: "1", transform: "scale(1) translateY(0)" },
        },
      },
      animation: {
        "toast-in": "toast-in 220ms ease-out",
        "modal-in": "modal-in 180ms ease-out",
      },
    },
  },
  plugins: [],
};

export default config;
