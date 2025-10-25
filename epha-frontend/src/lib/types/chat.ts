import type { Message, User } from '$lib/api/types';

export interface NotificationState {
	count: number;
	hasUnloadedUnread: boolean;
}

export interface ChatInterfaceProps {
	messages: Message[];
	loading: boolean;
	error: string | null;
	initialLoadDone: boolean;
	sendingError: string | null;
	notifications: NotificationState;
	currentUser: User | null;
	onSendMessage: (content: string) => void;
	onRetryLoad: () => void;
	onClearNotifications: () => void;
}

export interface WelcomeContentProps {
	onSignIn: () => void;
}