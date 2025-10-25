/**
 * Application configuration constants
 */

// Message polling and timing
export const MESSAGE_CONFIG = {
	/** Interval for checking new messages (milliseconds) */
	POLLING_INTERVAL: 3000,

	/** Delay after sending message before checking for new ones (milliseconds) */
	SEND_RETRY_DELAY: 100,
} as const;

// UI timing constants
export const UI_CONFIG = {
	/** Delay for hiding retry button state (milliseconds) */
	RETRY_BUTTON_DELAY: 2000,
} as const;

// Message storage limits
export const STORAGE_CONFIG = {
	/** Maximum number of messages to store in localStorage */
	MAX_STORED_MESSAGES: 1000,
} as const;