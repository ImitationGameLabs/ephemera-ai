
import type {
	User,
	CreateUserRequest,
	UserCredentials,
	Message,
	CreateMessageRequest,
	GetMessagesParams,
	MessageListResponse,
	OnlineStatus,
	ApiError,
	UsersList,
	Result
} from './types';

const API_BASE_URL = 'http://127.0.0.1:3000/api/v1';

class DialogueAtriumAPI {
	private async request<T>(
		endpoint: string,
		options: RequestInit = {}
	): Promise<Result<T>> {
		const url = `${API_BASE_URL}${endpoint}`;
		const method = options.method || 'GET';

		const config: RequestInit = {
			headers: {
				'Content-Type': 'application/json',
				...options.headers,
			},
			...options,
		};

		const response = await fetch(url, config)
			.then(response => response)
			.catch((err: unknown) => new Error(`Network Error: ${err}`));

		if (response instanceof Error) {
			return response
		}

		if (!response.ok) {
			// Try to parse error response body for more detailed error information
			try {
				const errorData = await response.json() as ApiError;
				if (errorData?.error?.details) {
					return new Error(String(errorData.error.details));
				} else if (errorData?.error?.message) {
					return new Error(errorData.error.message);
				}
			} catch {
				// If parsing fails, fall back to HTTP status
			}

			// Fallback to HTTP status if no detailed error information available
			return new Error(`HTTP ${response.status}: ${response.statusText}`);
		}

		const data = response.json()
			.then((data: T) => data)
			.catch((err: unknown) => {
				return new Error(`Failed to parse response body: ${err}`);
			});

		return data;
	}

	// User endpoints
	async createUser(userData: CreateUserRequest): Promise<Result<User>> {
		return this.request<User>('/users', {
			method: 'POST',
			body: JSON.stringify(userData),
		});
	}

	async getAllUsers(): Promise<Result<User[]>> {
		const result = await this.request<UsersList>('/users');
		if (result instanceof Error) {
			return result;
		}
		return result.users;
	}

	async getUser(username: string): Promise<Result<User>> {
		return this.request<User>(`/users/${username}`);
	}

	// Message endpoints
	async getMessages(params: GetMessagesParams = {}): Promise<Result<MessageListResponse>> {
		const searchParams = new URLSearchParams();

		if (params.sender) searchParams.append('sender', params.sender);
		if (params.limit) searchParams.append('limit', params.limit.toString());
		if (params.offset) searchParams.append('offset', params.offset.toString());

		const query = searchParams.toString();
		const endpoint = query ? `/messages?${query}` : '/messages';

		return this.request<MessageListResponse>(endpoint);
	}

	async sendMessage(messageData: CreateMessageRequest): Promise<Result<Message>> {
		return this.request<Message>('/messages', {
			method: 'POST',
			body: JSON.stringify(messageData),
		});
	}

	// Heartbeat endpoint
	async updateHeartbeat(credentials: UserCredentials): Promise<Result<OnlineStatus>> {
		return this.request<OnlineStatus>('/heartbeat', {
			method: 'PUT',
			body: JSON.stringify(credentials),
		});
	}
}

export const dialogueAtriumAPI = new DialogueAtriumAPI();