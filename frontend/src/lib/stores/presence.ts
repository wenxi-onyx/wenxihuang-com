import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { authStore } from './auth';

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

		// Use secure WebSocket if page is HTTPS
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		const host = window.location.hostname;
		// In dev, use port 8083 (backend port), in prod use current port
		const port = import.meta.env.DEV ? ':8083' : (window.location.port ? `:${window.location.port}` : '');
		const wsUrl = `${protocol}//${host}${port}/api/presence/ws`;

		try {
			console.log('Attempting to connect to WebSocket:', wsUrl);
			this.ws = new WebSocket(wsUrl);

			this.ws.onopen = () => {
				console.log('✅ Presence WebSocket connected');
				// Rejoin current page if we were on one
				if (this.currentPagePath) {
					console.log('Rejoining page:', this.currentPagePath);
					this.joinPage(this.currentPagePath);
				}
			};

			this.ws.onmessage = (event) => {
				try {
					const message: PresenceMessage = JSON.parse(event.data);
					console.log('📨 Received presence message:', message);
					this.handleMessage(message);
				} catch (error) {
					console.error('❌ Failed to parse presence message:', error);
				}
			};

			this.ws.onerror = (error) => {
				console.error('❌ WebSocket error:', error);
			};

			this.ws.onclose = (event) => {
				console.log('⚠️ Presence WebSocket closed. Code:', event.code, 'Reason:', event.reason);
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
			// Filter out current user from the list
			const currentUser = get(authStore).user;
			const otherUsers = message.users.filter((u) => u.user_id !== currentUser?.id);
			console.log('👥 Updating users:', otherUsers.length, 'other users on this page');
			this.users.set(otherUsers);
		}
	}

	private send(message: PresenceMessage) {
		if (this.ws?.readyState === WebSocket.OPEN) {
			console.log('📤 Sending message:', message.type);
			this.ws.send(JSON.stringify(message));
		} else {
			console.warn('⚠️ WebSocket not open, cannot send message:', message.type);
		}
	}

	public joinPage(pagePath: string) {
		console.log('🚪 Joining page:', pagePath);
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
