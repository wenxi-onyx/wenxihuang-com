import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';
import { authStore } from './auth';

// Enable detailed logging only in development
const DEBUG = import.meta.env.DEV;

export interface CursorPosition {
	x: number;
	y: number;
}

export interface UserPresence {
	user_id: string;
	username: string;
	page_path: string;
	cursor?: CursorPosition;
	color: string;
}

// Generate deterministic color from user_id (hash-based)
function getUserColor(userId: string): string {
	const colors = [
		'#3B82F6', // blue
		'#A855F7', // purple
		'#22C55E', // green
		'#EAB308', // yellow
		'#EF4444', // red
		'#EC4899', // pink
		'#0EA5E9', // cyan
		'#F97316', // orange
		'#8B5CF6', // violet
		'#10B981', // emerald
		'#F59E0B', // amber
		'#6366F1'  // indigo
	];

	// Simple hash function to get consistent color for same user
	let hash = 0;
	for (let i = 0; i < userId.length; i++) {
		hash = userId.charCodeAt(i) + ((hash << 5) - hash);
	}
	return colors[Math.abs(hash) % colors.length];
}

export interface PresenceMessage {
	type: 'join' | 'leave' | 'cursor_move' | 'presence_update';
	page_path?: string;
	x?: number;
	y?: number;
	users?: UserPresence[];
}

class PresenceStore {
	private ws: WebSocket | null = null;
	private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
	private currentPagePath: string = '';
	private cursorThrottle: ReturnType<typeof setTimeout> | null = null;

	// Store for users on current page
	public users = writable<UserPresence[]>([]);

	constructor() {
		if (browser) {
			// Subscribe to auth changes
			authStore.subscribe((auth) => {
				if (auth.user) {
					this.connect();
				} else {
					this.disconnect();
				}
			});
		}
	}

	private connect() {
		if (!browser || this.ws?.readyState === WebSocket.OPEN) return;

		// Clear any existing reconnection timeout
		if (this.reconnectTimeout) {
			clearTimeout(this.reconnectTimeout);
			this.reconnectTimeout = null;
		}

		// Get API URL from environment (same pattern as REST API client)
		const apiUrl = import.meta.env.VITE_API_URL || 'http://localhost:8083';
		// Convert HTTP(S) URL to WS(S)
		const wsUrl = apiUrl.replace(/^http/, 'ws') + '/api/presence/ws';

		try {
			if (DEBUG) console.log('Attempting to connect to WebSocket:', wsUrl);
			this.ws = new WebSocket(wsUrl);

			this.ws.onopen = () => {
				if (DEBUG) console.log('✅ Presence WebSocket connected');
				// Rejoin current page if we were on one
				if (this.currentPagePath) {
					if (DEBUG) console.log('Rejoining page:', this.currentPagePath);
					this.joinPage(this.currentPagePath);
				}
			};

			this.ws.onmessage = (event) => {
				try {
					const message: PresenceMessage = JSON.parse(event.data);
					this.handleMessage(message);
				} catch (error) {
					console.error('❌ Failed to parse presence message:', error);
				}
			};

			this.ws.onerror = (error) => {
				console.error('❌ WebSocket error:', error);
			};

			this.ws.onclose = (event) => {
				if (DEBUG) console.log('⚠️ Presence WebSocket closed. Code:', event.code, 'Reason:', event.reason);
				this.ws = null;
				// Try to reconnect after 3 seconds
				this.reconnectTimeout = setTimeout(() => this.connect(), 3000);
			};
		} catch (error) {
			console.error('❌ Failed to create WebSocket connection:', error);
		}
	}

	private disconnect() {
		if (this.reconnectTimeout) {
			clearTimeout(this.reconnectTimeout);
			this.reconnectTimeout = null;
		}

		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}

		this.users.set([]);
		this.currentPagePath = '';
	}

	private handleMessage(message: PresenceMessage) {
		if (message.type === 'presence_update' && message.users) {
			// Filter out current user from the list and assign colors
			const currentUser = get(authStore).user;
			const otherUsers = message.users
				.filter((u) => u.user_id !== currentUser?.id)
				.map((u) => ({
					...u,
					color: getUserColor(u.user_id) // Assign deterministic color
				}));
			this.users.set(otherUsers);
		}
	}

	private send(message: PresenceMessage) {
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(message));
		} else {
			if (DEBUG) console.warn('⚠️ WebSocket not open, cannot send message:', message.type);
		}
	}

	public joinPage(pagePath: string) {
		if (DEBUG) console.log('🚪 Joining page:', pagePath);
		this.currentPagePath = pagePath;
		this.send({
			type: 'join',
			page_path: pagePath
		});
	}

	public leavePage() {
		this.send({
			type: 'leave'
		});
		this.users.set([]);
		this.currentPagePath = '';
	}

	public updateCursor(x: number, y: number) {
		// Throttle cursor updates to avoid overwhelming the server
		if (this.cursorThrottle) return;

		this.send({
			type: 'cursor_move',
			x,
			y
		});

		this.cursorThrottle = setTimeout(() => {
			this.cursorThrottle = null;
		}, 50); // Send at most 20 updates per second
	}

	// Cleanup method (called when store is no longer needed)
	public cleanup() {
		if (this.cursorThrottle) {
			clearTimeout(this.cursorThrottle);
			this.cursorThrottle = null;
		}
		this.disconnect();
	}
}

export const presenceStore = new PresenceStore();
