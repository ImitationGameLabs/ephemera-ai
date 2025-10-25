import { writable } from 'svelte/store';
import { dialogueAtriumAPI } from '$lib/api/dialogue-atrium';
import type { UserCredentials } from '$lib/api/types';
import { HEARTBEAT_CONFIG, type HeartbeatStatus } from '$lib/config/heartbeat';

export class HeartbeatManager {
	private intervalId: ReturnType<typeof setInterval> | null = null;
	private failureCount = 0;
	private currentRetryDelay = 0;
	private credentials: UserCredentials | null = null;

	// Svelte store for heartbeat status
	public status = writable<HeartbeatStatus>('disconnected');
	public isRunning = writable<boolean>(false);

	/**
	 * Start heartbeat with given credentials
	 */
	public startHeartbeat(credentials: UserCredentials): void {
		if (this.intervalId) {
			this.stopHeartbeat();
		}

		this.credentials = credentials;
		this.failureCount = 0;
		this.currentRetryDelay = 0;
		this.isRunning.set(true);

		// Send immediate heartbeat
		this.sendHeartbeat();

		// Setup periodic heartbeat
		this.intervalId = setInterval(() => {
			this.sendHeartbeat();
		}, HEARTBEAT_CONFIG.interval);

		if (HEARTBEAT_CONFIG.debug) {
			console.log('Heartbeat started with interval:', HEARTBEAT_CONFIG.interval);
		}
	}

	/**
	 * Stop heartbeat
	 */
	public stopHeartbeat(): void {
		if (this.intervalId) {
			clearInterval(this.intervalId);
			this.intervalId = null;
		}

		this.credentials = null;
		this.failureCount = 0;
		this.currentRetryDelay = 0;
		this.isRunning.set(false);
		this.status.set('disconnected');

		if (HEARTBEAT_CONFIG.debug) {
			console.log('Heartbeat stopped');
		}
	}

	/**
	 * Send a single heartbeat
	 */
	private async sendHeartbeat(): Promise<void> {
		if (!this.credentials) {
			this.status.set('error');
			return;
		}

		try {
			this.status.set('connecting');

			const response = await dialogueAtriumAPI.updateHeartbeat(this.credentials);

			if (response instanceof Error) {
				throw response;
			}

			// Heartbeat successful
			this.failureCount = 0;
			this.currentRetryDelay = 0;
			this.status.set('connected');

			if (HEARTBEAT_CONFIG.debug) {
				console.log('Heartbeat successful:', response);
			}
		} catch (error) {
			this.failureCount++;
			this.status.set('error');

			if (HEARTBEAT_CONFIG.debug) {
				console.error('Heartbeat failed:', error);
			}

			// Check if we should stop due to too many failures
			if (this.failureCount >= HEARTBEAT_CONFIG.maxFailures) {
				this.handleTooManyFailures();
			} else {
				this.scheduleRetry();
			}
		}
	}

	/**
	 * Handle too many consecutive failures
	 */
	private handleTooManyFailures(): void {
		if (HEARTBEAT_CONFIG.debug) {
			console.log(`Too many heartbeat failures (${this.failureCount}), stopping heartbeat`);
		}

		this.stopHeartbeat();
	}

	/**
	 * Schedule retry with exponential backoff
	 */
	private scheduleRetry(): void {
		if (this.failureCount - 1 < HEARTBEAT_CONFIG.retryDelays.length) {
			this.currentRetryDelay = HEARTBEAT_CONFIG.retryDelays[this.failureCount - 1];
		} else {
			this.currentRetryDelay = Math.min(
				this.currentRetryDelay * 2,
				HEARTBEAT_CONFIG.maxBackoffDelay
			);
		}

		if (HEARTBEAT_CONFIG.debug) {
			console.log(`Scheduling heartbeat retry in ${this.currentRetryDelay}ms`);
		}

		setTimeout(() => {
			this.sendHeartbeat();
		}, this.currentRetryDelay);
	}

	/**
	 * Check if heartbeat is currently running
	 */
	public get isActive(): boolean {
		return this.intervalId !== null;
	}

	/**
	 * Get current failure count
	 */
	public get consecutiveFailures(): number {
		return this.failureCount;
	}
}

// Create singleton instance
export const heartbeatManager = new HeartbeatManager();