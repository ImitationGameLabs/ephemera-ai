export const HEARTBEAT_CONFIG = {
	// Interval in milliseconds (30 seconds)
	interval: 30000,

	// Timeout for each heartbeat request in milliseconds
	timeout: 5000,

	// Maximum number of consecutive failures before switching to offline mode
	maxFailures: 3,

	// Backoff strategy for retries (in milliseconds)
	retryDelays: [1000, 2000, 5000, 10000, 30000],

	// Maximum backoff delay
	maxBackoffDelay: 30000,

	// Whether to enable heartbeat debugging logs
	debug: false
} as const;

export type HeartbeatStatus = 'connected' | 'connecting' | 'disconnected' | 'error';