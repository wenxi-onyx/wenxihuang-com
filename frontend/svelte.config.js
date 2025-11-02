import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter({
			// Enable brotli and gzip compression for all assets
			precompress: true
		}),
		prerender: {
			// Handle missing routes gracefully during prerendering
			handleHttpError: ({ path, referrer, message }) => {
				// Ignore 404 errors for routes that don't exist yet
				if (message.includes('404')) {
					console.warn(`Skipping ${path} (linked from ${referrer})`);
					return;
				}
				throw new Error(message);
			}
		}
	}
};

export default config;
