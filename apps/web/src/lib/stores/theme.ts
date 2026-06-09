import { writable } from 'svelte/store';
import { browser } from '$app/environment';

type Theme = 'dark' | 'light';

// Initialize theme from localStorage or default to dark
const getInitialTheme = (): Theme => {
	if (!browser) return 'dark';

	const stored = localStorage.getItem('theme');
	if (stored === 'dark' || stored === 'light') {
		return stored;
	}

	// Default to dark mode (French brutalism)
	return 'dark';
};

const createThemeStore = () => {
	const { subscribe, set } = writable<Theme>(getInitialTheme());

	return {
		subscribe,
		toggle: () => {
			if (!browser) return;

			const current = document.documentElement.getAttribute('data-theme') as Theme;
			const newTheme: Theme = current === 'dark' ? 'light' : 'dark';

			document.documentElement.setAttribute('data-theme', newTheme);
			localStorage.setItem('theme', newTheme);
			set(newTheme);
		},
		syncTheme: (theme: Theme) => {
			if (!browser) return;
			set(theme);
		},
		init: () => {
			// Deprecated: Theme is now initialized in app.html inline script
			// Kept for backwards compatibility
			if (!browser) return;

			const theme = getInitialTheme();
			document.documentElement.setAttribute('data-theme', theme);
			set(theme);
		}
	};
};

export const theme = createThemeStore();
