/** Errors mirror the original backend's AuthError -> { error: message } contract. */
export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export const invalidCredentials = () => new ApiError(401, 'Invalid username or password');
export const unauthorized = () => new ApiError(401, 'Authentication required');
export const forbidden = () => new ApiError(403, 'Insufficient permissions');
export const sessionExpired = () => new ApiError(401, 'Session expired');
export const databaseError = () => new ApiError(500, 'Database error');
export const usernameTaken = () => new ApiError(409, 'Username already taken');
export const invalidInput = (msg: string) => new ApiError(400, msg);
