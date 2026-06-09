import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		// Dev: forward API calls to the local Fastify server (apps/api, port 8080)
		proxy: {
			'/api': 'http://localhost:8080',
			'/health': 'http://localhost:8080'
		}
	},
	build: {
		cssCodeSplit: true,
		cssMinify: true,
		rollupOptions: {
			output: {
				manualChunks: {
					'svelte-runtime': ['svelte', 'svelte/store']
				},
				// Optimize chunk names for better caching
				chunkFileNames: '_app/immutable/chunks/[name]-[hash].js',
				assetFileNames: '_app/immutable/assets/[name]-[hash][extname]'
			}
		},
		// Aggressive minification for production
		minify: 'esbuild',
		target: 'es2020',
		// Additional optimizations
		reportCompressedSize: true,
		chunkSizeWarningLimit: 500,
		sourcemap: false
	},
	// Optimize dependencies
	optimizeDeps: {
		include: ['svelte', 'svelte/store']
	}
});
