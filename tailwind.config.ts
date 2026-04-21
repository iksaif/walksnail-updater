import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        brand: {
          bg: "#0F172A",
          fg: "#F8FAFC",
          accent: "#F59E0B",
          // Bumped from slate-600 (#475569, unreadable on #0F172A) to slate-400.
          muted: "#94A3B8",
          // Even lighter for secondary text that sits on dark panels.
          dim: "#CBD5E1",
          danger: "#DC2626",
          warn: "#F59E0B",
          ok: "#10B981",
          info: "#38BDF8",
        },
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "Inter",
          "sans-serif",
        ],
        mono: ["ui-monospace", "SFMono-Regular", "Menlo", "monospace"],
      },
      keyframes: {
        "progress-indeterminate": {
          "0%": { transform: "translateX(-100%)" },
          "100%": { transform: "translateX(400%)" },
        },
      },
      animation: {
        "progress-indeterminate": "progress-indeterminate 1.4s ease-in-out infinite",
      },
    },
  },
  plugins: [],
} satisfies Config;
