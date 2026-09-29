import { resolve } from 'node:path';

import { getViteConfig } from 'astro/config';

export default getViteConfig({
  envDir: false,
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@qafiyah/config': resolve(__dirname, '../../config.ts'),
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
  },
});
