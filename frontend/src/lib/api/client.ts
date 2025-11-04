// API client for the backend

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8080';

export interface User {
    id: string;
    username: string;
    first_name: string | null;
    last_name: string | null;
    role: 'admin' | 'user';
}

export interface AuthResponse {
    user: User;
}

export interface LoginRequest {
    username: string;
    password: string;
}

export interface RegisterRequest {
    username: string;
    password: string;
    role: 'admin' | 'user';
}

async function apiCall<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const response = await fetch(`${API_BASE}${endpoint}`, {
        ...options,
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', ...(options.headers || {}) },
    });

    if (!response.ok) {
        const error = await response.json().catch(() => ({ error: response.statusText }));
        throw new Error(error.error || 'Request failed');
    }
    return response.json();
}

// Auth API methods
export const authApi = {
    async login(credentials: LoginRequest): Promise<AuthResponse> {
        return apiCall<AuthResponse>('/api/auth/login', {
            method: 'POST',
            body: JSON.stringify(credentials),
        });
    },

    async logout(): Promise<{ message: string }> {
        return apiCall<{ message: string }>('/api/auth/logout', {
            method: 'POST',
        });
    },

    async getCurrentUser(): Promise<AuthResponse> {
        return apiCall<AuthResponse>('/api/auth/me', {
            method: 'GET',
        });
    },

    async register(data: RegisterRequest): Promise<AuthResponse> {
        return apiCall<AuthResponse>('/api/auth/register', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    },
};

// User profile API methods
export interface UpdateProfileRequest {
    username: string;
    first_name?: string | null;
    last_name?: string | null;
}

export interface ChangePasswordRequest {
    current_password: string;
    new_password: string;
}

export interface ProfileResponse {
    user: User;
}

export const userApi = {
    async getProfile(): Promise<ProfileResponse> {
        return apiCall<ProfileResponse>('/api/user/profile', {
            method: 'GET',
        });
    },

    async updateProfile(data: UpdateProfileRequest): Promise<ProfileResponse> {
        return apiCall<ProfileResponse>('/api/user/profile', {
            method: 'PUT',
            body: JSON.stringify(data),
        });
    },

    async changePassword(data: ChangePasswordRequest): Promise<{ message: string }> {
        return apiCall<{ message: string }>('/api/user/change-password', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    },
};

// Admin API methods
export interface CreateUserRequest {
    username: string;
    password: string;
    first_name?: string | null;
    last_name?: string | null;
    role: 'admin' | 'user';
}

export const adminApi = {
    async createUser(data: CreateUserRequest): Promise<{ message: string; user: User }> {
        return apiCall<{ message: string; user: User }>('/api/admin/users', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    },
};
