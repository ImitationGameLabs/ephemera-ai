/**
 * Auto-scroll action for chat interfaces
 * Intelligently scrolls to bottom when new content is added
 * but respects user scrolling behavior
 */

interface AutoScrollOptions {
	threshold?: number; // Distance from bottom to consider "near bottom" (default: 100)
	smooth?: boolean; // Whether to use smooth scrolling (default: true)
}

export function autoScroll(node: HTMLElement, options: AutoScrollOptions = {}) {
	const { threshold = 100, smooth = true } = options;
	let isUserScrolling = false;
	let scrollTimeout: ReturnType<typeof setTimeout>;

	// Handle scroll events to detect when user is manually scrolling
	const handleScroll = () => {
		isUserScrolling = true;
		clearTimeout(scrollTimeout);

		// Consider user not scrolling after 1 second of inactivity
		scrollTimeout = setTimeout(() => {
			isUserScrolling = false;
		}, 1000);
	};

	// Scroll to bottom if appropriate
	const scrollToBottom = (force = false) => {
		if (!node) return;

		const scrollHeight = node.scrollHeight;
		const scrollTop = node.scrollTop;
		const clientHeight = node.clientHeight;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

		// Only auto-scroll if user is not manually scrolling or if forced
		if (!isUserScrolling || force || distanceFromBottom < threshold) {
			node.scrollTo({
				top: scrollHeight,
				behavior: smooth && !force ? 'smooth' : 'auto'
			});
		}
	};

	// Public method to scroll to bottom immediately
	const scrollToBottomImmediate = () => scrollToBottom(true);

	// Check if user is near bottom
	const isNearBottom = () => {
		const { scrollHeight, scrollTop, clientHeight } = node;
		return scrollHeight - scrollTop - clientHeight < threshold;
	};

	// Set up scroll listener
	node.addEventListener('scroll', handleScroll, { passive: true });

	// Return action API
	return {
		update: (newOptions: AutoScrollOptions = {}) => {
			Object.assign(options, newOptions);
		},
		destroy: () => {
			node.removeEventListener('scroll', handleScroll);
			clearTimeout(scrollTimeout);
		},
		api: {
			scrollToBottom: scrollToBottomImmediate,
			isNearBottom,
			isUserScrolling: () => isUserScrolling
		}
	};
}