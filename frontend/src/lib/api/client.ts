// API client for the backend

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8083';

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

export interface CreateEloConfigRequest {
    version_name: string;
    k_factor: number;
    starting_elo: number;
    base_k_factor?: number;
    new_player_k_bonus?: number;
    new_player_bonus_period?: number;
    description?: string;
}

export interface UpdateEloConfigRequest {
    k_factor?: number;
    starting_elo?: number;
    base_k_factor?: number;
    new_player_k_bonus?: number;
    new_player_bonus_period?: number;
    description?: string;
}

export interface EloConfiguration {
    id: string;
    version_name: string;
    k_factor: number;
    starting_elo: number;
    base_k_factor: number | null;
    new_player_k_bonus: number | null;
    new_player_bonus_period: number | null;
    description: string | null;
    is_active: boolean;
    created_at: string;
}

export type JobStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface Job {
    id: string;
    job_type: string;
    status: JobStatus;
    progress: number;
    total_items: number | null;
    processed_items: number;
    result_data: any | null;
    created_by: string | null;
    created_at: string;
    started_at: string | null;
    completed_at: string | null;
}

export interface Player {
    id: string;
    name: string;
    current_elo: number;
    is_active: boolean;
    created_at: string;
    updated_at: string;
}

export interface PlayerWithStats extends Player {
    games_played: number;
    wins: number;
    losses: number;
}

export interface EloHistoryPoint {
    game_id: string;
    elo_before: number;
    elo_after: number;
    elo_version: string;
    created_at: string;
}

export interface Season {
    id: string;
    name: string;
    description: string | null;
    start_date: string;
    starting_elo: number;
    k_factor: number;
    base_k_factor: number | null;
    new_player_k_bonus: number | null;
    new_player_bonus_period: number | null;
    is_active: boolean;
    created_at: string;
}

export interface CreateSeasonRequest {
    name: string;
    description?: string;
    start_date: string;
    starting_elo: number;
    k_factor: number;
    base_k_factor?: number;
    new_player_k_bonus?: number;
    new_player_bonus_period?: number;
}

export interface PlayerSeasonStats {
    player_id: string;
    player_name: string;
    current_elo: number;
    games_played: number;
    wins: number;
    losses: number;
    win_rate: number;
    is_active: boolean;
}

export interface SeasonPlayer {
    player_id: string;
    player_name: string;
    is_included: boolean;
    is_active: boolean;
}

export const adminApi = {
    async createUser(data: CreateUserRequest): Promise<{ message: string; user: User }> {
        return apiCall<{ message: string; user: User }>('/api/admin/users', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    },

    async createEloConfiguration(data: CreateEloConfigRequest): Promise<EloConfiguration> {
        return apiCall<EloConfiguration>('/api/admin/elo-configurations', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    },

    async listEloConfigurations(): Promise<EloConfiguration[]> {
        return apiCall<EloConfiguration[]>('/api/admin/elo-configurations', {
            method: 'GET',
        });
    },

    async activateEloConfiguration(versionName: string): Promise<{ message: string }> {
        return apiCall<{ message: string }>(`/api/admin/elo-configurations/${versionName}/activate`, {
            method: 'POST',
        });
    },

    async recalculateElo(versionName: string): Promise<{ message: string; job_id: string; version: string }> {
        return apiCall<{ message: string; job_id: string; version: string }>(`/api/admin/elo-configurations/${versionName}/recalculate`, {
            method: 'POST',
        });
    },

    async getJobStatus(jobId: string): Promise<Job> {
        return apiCall<Job>(`/api/admin/jobs/${jobId}`, {
            method: 'GET',
        });
    },

    async updateEloConfiguration(versionName: string, data: UpdateEloConfigRequest): Promise<EloConfiguration> {
        return apiCall<EloConfiguration>(`/api/admin/elo-configurations/${versionName}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        });
    },

    async deleteEloConfiguration(versionName: string): Promise<{ message: string }> {
        return apiCall<{ message: string }>(`/api/admin/elo-configurations/${versionName}`, {
            method: 'DELETE',
        });
    },

    async togglePlayerActive(playerId: string): Promise<Player> {
        return apiCall<Player>(`/api/admin/players/${playerId}/toggle-active`, {
            method: 'POST',
        });
    },

    // Season management
    async createSeason(data: CreateSeasonRequest): Promise<Season> {
        return apiCall<Season>('/api/admin/seasons', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    },

    async activateSeason(seasonId: string): Promise<{ message: string }> {
        return apiCall<{ message: string }>(`/api/admin/seasons/${seasonId}/activate`, {
            method: 'POST',
        });
    },

    async recalculateSeason(seasonId: string): Promise<{ message: string }> {
        return apiCall<{ message: string }>(`/api/admin/seasons/${seasonId}/recalculate`, {
            method: 'POST',
        });
    },

    async deleteSeason(seasonId: string): Promise<{ message: string }> {
        return apiCall<{ message: string }>(`/api/admin/seasons/${seasonId}`, {
            method: 'DELETE',
        });
    },

    // Season player management
    async getSeasonPlayers(seasonId: string): Promise<SeasonPlayer[]> {
        return apiCall<SeasonPlayer[]>(`/api/admin/seasons/${seasonId}/players`, {
            method: 'GET',
        });
    },

    async getAvailablePlayers(seasonId: string): Promise<SeasonPlayer[]> {
        return apiCall<SeasonPlayer[]>(`/api/admin/seasons/${seasonId}/available-players`, {
            method: 'GET',
        });
    },

    async addPlayerToSeason(seasonId: string, playerId: string): Promise<{ message: string }> {
        return apiCall<{ message: string }>(`/api/admin/seasons/${seasonId}/players/add`, {
            method: 'POST',
            body: JSON.stringify({ player_id: playerId }),
        });
    },

    async removePlayerFromSeason(seasonId: string, playerId: string): Promise<{ message: string }> {
        return apiCall<{ message: string }>(`/api/admin/seasons/${seasonId}/players/remove`, {
            method: 'POST',
            body: JSON.stringify({ player_id: playerId }),
        });
    },
};

// Public Seasons API methods
export const seasonsApi = {
    async listSeasons(): Promise<Season[]> {
        return apiCall<Season[]>('/api/seasons', {
            method: 'GET',
        });
    },

    async getActiveSeason(): Promise<Season | null> {
        return apiCall<Season | null>('/api/seasons/active', {
            method: 'GET',
        });
    },

    async getSeason(seasonId: string): Promise<Season> {
        return apiCall<Season>(`/api/seasons/${seasonId}`, {
            method: 'GET',
        });
    },

    async getSeasonLeaderboard(seasonId: string): Promise<PlayerSeasonStats[]> {
        return apiCall<PlayerSeasonStats[]>(`/api/seasons/${seasonId}/leaderboard`, {
            method: 'GET',
        });
    },
};

// Public Players API methods
export const playersApi = {
    async listPlayers(): Promise<PlayerWithStats[]> {
        return apiCall<PlayerWithStats[]>('/api/players', {
            method: 'GET',
        });
    },

    async getPlayerHistory(playerId: string): Promise<EloHistoryPoint[]> {
        return apiCall<EloHistoryPoint[]>(`/api/players/${playerId}/history`, {
            method: 'GET',
        });
    },
};
