/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,jsx,ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // CYNIC Sefirot Dog Colors (φ-weighted)
        analyst: '#8B5CF6',      // Purple — analytical
        architect: '#3B82F6',    // Blue — blueprint
        cartographer: '#06B6D4', // Cyan — mapping
        cynic: '#EF4444',        // Red — truth
        deployer: '#10B981',     // Green — action
        guardian: '#F59E0B',     // Amber — safety
        janitor: '#6366F1',      // Indigo — cleanup
        oracle: '#EC4899',       // Pink — foresight
        sage: '#F97316',         // Orange — wisdom
        scholar: '#8B5CF6',      // Purple — learning
        scout: '#14B8A6',        // Teal — discovery
        // Verdict colors
        howl: '#10B981',         // Green — excellent
        wag: '#3B82F6',          // Blue — good
        growl: '#F59E0B',        // Amber — warning
        bark: '#EF4444',         // Red — critical
      },
      fontFamily: {
        mono: ['Courier New', 'monospace'],
      },
      animation: {
        pulse: 'pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        glow: 'glow 3s ease-in-out infinite',
      },
      keyframes: {
        glow: {
          '0%, 100%': { boxShadow: '0 0 5px rgba(34, 197, 94, 0.5)' },
          '50%': { boxShadow: '0 0 20px rgba(34, 197, 94, 0.8)' },
        }
      }
    },
  },
  plugins: [],
}
