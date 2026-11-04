import { defineConfig } from '@inlang/paraglide-js';

// What Paraglide compiles, read by both of the ways it runs: the Vite plugin
// in vite.config.ts, and `paraglide-js compile` in `npm run check`, which
// writes the output svelte-check reads on a checkout that has never built
// (#532). Written once here so the two cannot compile differently. The layout
// of the output is the one thing each chooses for itself.
export default defineConfig({
	outdir: './src/lib/paraglide',
	strategy: ['localStorage', 'preferredLanguage', 'baseLocale'],
	emitTsDeclarations: true,
});
