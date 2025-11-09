import { writable } from 'svelte/store';
import { browser } from '$app/environment';

const DEBUG = import.meta.env.DEV;

export interface CommentWithAuthor {
	id: string;
	plan_id: string;
	plan_version: number;
	author_id: string;
	line_start: number;
	line_end: number;
	comment_text: string;
	is_resolved: boolean;
	resolved_at: string | null;
	resolved_by: string | null;
	resolution_action: 'accepted' | 'rejected' | null;
	created_at: string;
	updated_at: string;
	author_username: string;
	author_first_name: string | null;
	author_last_name: string | null;
}

export interface CommentMessage {
	type: 'comment_added' | 'comment_updated' | 'comment_deleted';
	plan_id: string;
	comment?: CommentWithAuthor; // Full comment object from backend (for added/updated)
	comment_id?: string; // Comment ID (for deleted)
}

class PlanCommentsStore {
	private ws: WebSocket | null = null;
	private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
	private currentPlanId: string | null = null;
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 5;

	// Callback for when comments are updated
	private onUpdateCallback: ((message: CommentMessage) => void) | null = null;
	private onErrorCallback: ((error: string) => void) | null = null;

	constructor() {
		// WebSocket will be initialized when subscribeToPlan is called
	}

	private connect(planId: string) {
		if (!browser) return;

		// Prevent duplicate connections
		if (this.ws?.readyState === WebSocket.OPEN ||
			this.ws?.readyState === WebSocket.CONNECTING) {
			return;
		}

		// Clean up any existing connection
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}

		if (this.reconnectTimeout) {
			clearTimeout(this.reconnectTimeout);
			this.reconnectTimeout = null;
		}

		const apiUrl = import.meta.env.VITE_API_URL || 'http://localhost:8083';
		const wsUrl = apiUrl.replace(/^http/, 'ws') + `/api/plans/${planId}/ws`;

		try {
			if (DEBUG) console.log('Connecting to plan WebSocket:', wsUrl);
			this.ws = new WebSocket(wsUrl);

			this.ws.onopen = () => {
				if (DEBUG) console.log('✅ Plan comments WebSocket connected');
				this.reconnectAttempts = 0;
			};

			this.ws.onmessage = (event) => {
				try {
					const message: CommentMessage = JSON.parse(event.data);
					this.handleMessage(message);
				} catch (error) {
					console.error('❌ Failed to parse plan comment message:', error);
				}
			};

			this.ws.onerror = (error) => {
				console.error('❌ Plan WebSocket error:', error);
			};

			this.ws.onclose = (event) => {
				if (DEBUG) console.log('⚠️ Plan WebSocket closed. Code:', event.code);
				this.ws = null;

				// FIX #11: Capture planId at time of scheduling to avoid stale reference
				const planIdToReconnect = this.currentPlanId;

				// Attempt to reconnect with exponential backoff
				if (planIdToReconnect && this.reconnectAttempts < this.maxReconnectAttempts) {
					const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 10000);
					this.reconnectAttempts++;
					if (DEBUG) console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
					this.reconnectTimeout = setTimeout(() => {
						// FIX #11: Only reconnect if we're still subscribed to the same plan
						if (this.currentPlanId === planIdToReconnect) {
							this.connect(planIdToReconnect);
						}
					}, delay);
				} else if (this.reconnectAttempts >= this.maxReconnectAttempts) {
					// FIX #7 & #10: Clear callback and notify error
					const errorMsg = 'WebSocket connection failed after maximum retry attempts';
					console.error('❌', errorMsg);
					if (this.onErrorCallback) {
						this.onErrorCallback(errorMsg);
					}
					this.onUpdateCallback = null;
				}
			};
		} catch (error) {
			console.error('❌ Failed to create plan WebSocket connection:', error);
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

		this.currentPlanId = null;
		this.reconnectAttempts = 0;
	}

	private handleMessage(message: CommentMessage) {
		if (DEBUG) console.log('Received plan comment message:', message);

		if (this.onUpdateCallback) {
			this.onUpdateCallback(message);
		}
	}

	public subscribeToPlan(
		planId: string,
		onUpdate: (message: CommentMessage) => void,
		onError?: (error: string) => void
	) {
		if (this.currentPlanId && this.currentPlanId !== planId) {
			this.disconnect();
		}

		this.currentPlanId = planId;
		this.onUpdateCallback = onUpdate;
		this.onErrorCallback = onError || null;
		this.connect(planId);
	}

	public unsubscribe() {
		this.onUpdateCallback = null;
		this.onErrorCallback = null;
		this.disconnect();
	}

	public cleanup() {
		this.disconnect();
	}
}

export const planCommentsStore = new PlanCommentsStore();
