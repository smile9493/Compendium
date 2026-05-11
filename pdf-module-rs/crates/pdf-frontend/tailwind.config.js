/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: ["./src/**/*.rs", "./index.html"],
  theme: {
    extend: {
      colors: {
        surface: '#1e293b',
        'surface-hover': '#334155',
        accent: '#3b82f6',
        'accent-hover': '#2563eb',
        'accent-teal': '#0d9488',
        muted: '#94a3b8',
        'muted-deep': '#64748b',
      },
      borderRadius: {
        card: '12px',
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [],
}