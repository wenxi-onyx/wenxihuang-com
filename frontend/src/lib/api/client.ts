// API client for the backend

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8080';

export async function apiCall<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const response = await fetch(`${API_BASE}${endpoint}`, {
        ...options,
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', ...(options.headers || {}) },
    });

    if (!response.ok) throw new Error(`API Error: ${response.statusText}`);
    return response.json();
}
