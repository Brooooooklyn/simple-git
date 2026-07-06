import { defineConfig } from 'vite'
import { voidPlugin } from 'void'
import { voidReact } from '@void/react/plugin'
import { voidMarkdown } from '@void/md/plugin'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [
    voidPlugin(),
    voidReact(),
    voidMarkdown(), // enforce:'pre', auto-detects React → MUST come after voidReact()
    tailwindcss(),
  ],
})
