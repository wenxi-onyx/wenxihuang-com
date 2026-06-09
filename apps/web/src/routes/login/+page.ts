// Disable prerendering for the login page
// Login requires client-side authentication and API calls
export const prerender = false;
export const ssr = false; // Client-side only for auth
