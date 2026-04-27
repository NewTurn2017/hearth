import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://hearth.codewithgenie.com',
  trailingSlash: 'never',
  build: { format: 'directory' },
  i18n: {
    defaultLocale: 'ko',
    locales: ['ko', 'en'],
    routing: { prefixDefaultLocale: true, redirectToDefaultLocale: false },
    fallback: { en: 'ko' },
  },
});
