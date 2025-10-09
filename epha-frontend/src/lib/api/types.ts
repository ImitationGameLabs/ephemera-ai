// TypeScript interfaces for Dialogue Atrium API
export interface User {
	name: string;
	bio: string;
	status: UserStatus;
	message_height: number;
	created_at: string;
}

export interface UserStatus {
	online: boolean;
	last_seen: string;
}

export interface OnlineStatus {
	online: boolean;
	last_seen: string;
}

export interface CreateUserRequest {
	name: string;
	bio: string;
	password: string;
}

export interface UpdateProfileRequest {
	current_password?: string;
	bio?: string;
	new_password?: string;
}

export interface PasswordAuth {
	password: string;
}

export interface UserCredentials {
	username: string;
	password: string;
}

export interface UsersList {
	users: User[];
}

export interface Message {
	id: number;
	content: string;
	sender: string;
	created_at: string;
}

export interface CreateMessageRequest {
	content: string;
	username: string;
	password: string;
}

export interface MessageListResponse {
	messages: Message[];
}

export interface ApiError {
	error: {
		code: string;
		message: string;
		details?: object;
	};
}

export interface GetMessagesParams {
	sender?: string;
	limit?: number;
	offset?: number;
}

export type Result<T, E = Error> = T | E;