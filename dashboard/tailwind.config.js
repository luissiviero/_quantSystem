// @file: dashboard\tailwind.config.js
// @description: Defines the content sources for Tailwind CSS to scan.
// @author: LAS.

/** @type {import('tailwindcss').Config} */
export default {
  // #
  // # CONTENT SOURCES
  // #
  // This array tells Tailwind to scan index.html and all JS/TS/JSX/TSX files
  // in the src folder for utility classes.
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}