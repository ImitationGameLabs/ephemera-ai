/**
 * Chat-specific scroll manager that combines auto-scroll with UI state management
 */

export function chatScrollManager(node: HTMLElement) {
	let showScrollToBottom = false;
	let isUserScrolling = false;
	let scrollTimeout: ReturnType<typeof setTimeout>;
	let onClearNotifications: (() => void) | undefined;

	// Handle scroll events for loading more messages and scroll-to-bottom button
	const handleScroll = () => {
		isUserScrolling = true;
		clearTimeout(scrollTimeout);

		const { scrollTop, scrollHeight, clientHeight } = node;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

		// Show scroll to bottom button if not near bottom
		showScrollToBottom = distanceFromBottom > 200;

		// Clear notifications if user scrolls to bottom (viewing latest messages)
		if (distanceFromBottom < 10) {
			onClearNotifications?.();
		}

		// Consider user not scrolling after 1 second of inactivity
		scrollTimeout = setTimeout(() => {
			isUserScrolling = false;
		}, 1000);
	};

	// Auto-scroll to bottom when new content is added or when requested
	const scrollToBottom = (force = false) => {
		if (!node) return;

		const scrollHeight = node.scrollHeight;
		const scrollTop = node.scrollTop;
		const clientHeight = node.clientHeight;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

		// Only auto-scroll if user is not manually scrolling or if forced or near bottom
		if (!isUserScrolling || force || distanceFromBottom < 100) {
			node.scrollTo({
				top: scrollHeight,
				behavior: !force && !isUserScrolling ? 'smooth' : 'auto'
			});
		}
	};

	// Trigger auto-scroll when messages change
	const observer = new MutationObserver(() => {
		scrollToBottom();
	});

	// Set up listeners
	node.addEventListener('scroll', handleScroll, { passive: true });
	observer.observe(node, {
		childList: true,
		subtree: true,
		characterData: true
	});

	// Auto-scroll to bottom initially
	setTimeout(() => scrollToBottom(true), 100);

	return {
		update: (props: {
			messages?: any[];
			onClearNotifications?: () => void;
		}) => {
			if (props.onClearNotifications) {
				onClearNotifications = props.onClearNotifications;
			}

			// Auto-scroll when messages change
			if (props.messages) {
				scrollToBottom();
			}
		},
		destroy: () => {
			node.removeEventListener('scroll', handleScroll);
			observer.disconnect();
			clearTimeout(scrollTimeout);
		},
		api: {
			scrollToBottom: () => scrollToBottom(true),
			isNearBottom: () => {
				const { scrollHeight, scrollTop, clientHeight } = node;
				return scrollHeight - scrollTop - clientHeight < 100;
			},
			showScrollToBottomButton: () => showScrollToBottom
		}
	};
}