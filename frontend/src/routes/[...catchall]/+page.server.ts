import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

// Catch-all route that redirects any unmatched URLs to root
export const load: PageServerLoad = async () => {
	// Redirect to root with 307 (temporary redirect)
	// Using temporary redirect is best practice for 404s since the page might exist in the future
	throw redirect(307, '/');
};
