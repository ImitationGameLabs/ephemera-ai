import { writable } from 'svelte/store';
import { dialogueAtriumAPI } from '$lib/api/dialogue-atrium';
import type { User, CreateUserRequest, UserCredentials } from '$lib/api/types';
import { heartbeatManager } from '$lib/services/heartbeat';

export interface AuthenticatedUser {
	user: User;
	credentials: UserCredentials;
}

export interface AuthState {
	authenticatedUser: AuthenticatedUser | null;
}

const defaultAuthState: AuthState = {
	authenticatedUser: null,
};

// Create Svelte store
const authStore = writable<AuthState>(structuredClone(defaultAuthState));

export const auth = {
	// Subscribe to store changes
	subscribe: authStore.subscribe,

	// Get current state
	get state(): AuthState {
		let currentState: AuthState = defaultAuthState;
		authStore.subscribe(value => currentState = value)();
		return currentState;
	},

	async login(username: string, password: string): Promise<Error | null> {
		// Verify credentials by sending heartbeat
		const heartbeatResponse = await dialogueAtriumAPI.updateHeartbeat({
			username,
			password,
		});

		if (heartbeatResponse instanceof Error) {
			return heartbeatResponse;
		}

		// Get user details
		const user = await dialogueAtriumAPI.getUser(username);
		if (user instanceof Error) {
			return user;
		}

		const credentials: UserCredentials = { username, password };

		authStore.update(state => ({
			...state,
			authenticatedUser: {
				user,
				credentials
			}
		}));

		// Store in localStorage for persistence
		if (typeof window !== 'undefined') {
			localStorage.setItem('auth_user', JSON.stringify(user));
			localStorage.setItem('auth_password', password);
		}

		// Start heartbeat service
		heartbeatManager.startHeartbeat(credentials);

		return null;
	},

	async register(userData: CreateUserRequest): Promise<Error | null> {
		const result = await dialogueAtriumAPI.createUser(userData);
		if (result instanceof Error) {
			return result;
		}

		return null;
	},

	logout(): void {
		// Stop heartbeat service
		heartbeatManager.stopHeartbeat();

		authStore.update(state => ({
			...state,
			authenticatedUser: null
		}));

		// Clear localStorage and stop heartbeat
		if (typeof window !== 'undefined') {
			localStorage.removeItem('auth_user');
			localStorage.removeItem('auth_password');
		}
		heartbeatManager.stopHeartbeat();
	},

	async restoreSession(): Promise<void> {
		if (typeof window === 'undefined') return;

		const storedUser = localStorage.getItem('auth_user');
		const storedPassword = localStorage.getItem('auth_password');

		if (storedUser && storedPassword) {
			try {
				const user = JSON.parse(storedUser);
				const username = user.name;

				// Verify credentials are still valid
				const error = await this.login(username, storedPassword);
				if (error) {
					// Clear invalid stored data
					this.logout();
				} else {
					// Session restored successfully, heartbeat already started by login()
				}
			} catch (error) {
				// Clear corrupted stored data
				this.logout();
			}
		}
	},
};