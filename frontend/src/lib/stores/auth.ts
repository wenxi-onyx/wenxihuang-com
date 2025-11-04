import { writable } from 'svelte/store';
import { authApi, type User } from '$lib/api/client';
import { goto } from '$app/navigation';

export interface AuthState {
    user: User | null;
    loading: boolean;
}

function createAuthStore() {
    const { subscribe, set, update } = writable<AuthState>({
        user: null,
        loading: false, // Start as false - auth check happens in background
    });

    return {
        subscribe,

        async checkAuth() {
            try {
                const response = await authApi.getCurrentUser();
                set({ user: response.user, loading: false });
                return response.user;
            } catch (error) {
                set({ user: null, loading: false });
                return null;
            }
        },

        async login(username: string, password: string) {
            try {
                const response = await authApi.login({ username, password });
                set({ user: response.user, loading: false });
                goto('/');
                return { success: true, user: response.user };
            } catch (error) {
                return {
                    success: false,
                    error: error instanceof Error ? error.message : 'Login failed'
                };
            }
        },

        async logout() {
            try {
                await authApi.logout();
                set({ user: null, loading: false });
                goto('/');
            } catch (error) {
                console.error('Logout failed:', error);
                // Clear user anyway
                set({ user: null, loading: false });
                goto('/');
            }
        },

        async register(username: string, password: string, role: 'admin' | 'user') {
            try {
                const response = await authApi.register({ username, password, role });
                return { success: true, user: response.user };
            } catch (error) {
                return {
                    success: false,
                    error: error instanceof Error ? error.message : 'Registration failed'
                };
            }
        },

        updateUser(user: User) {
            update((state) => ({ ...state, user }));
        },
    };
}

export const authStore = createAuthStore();
