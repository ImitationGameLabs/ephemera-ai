/**
 * Auto-resize action for textarea elements
 * Automatically adjusts height based on content within specified limits
 */

interface AutoResizeOptions {
	minHeight?: number; // Minimum height in pixels (default: 44)
	maxHeight?: number; // Maximum height in pixels (default: 120)
}

export function autoResize(node: HTMLTextAreaElement, options: AutoResizeOptions = {}) {
	const { minHeight = 44, maxHeight = 120 } = options;

	// Resize textarea to fit content
	const resize = () => {
		// Reset height to get accurate scrollHeight
		node.style.height = 'auto';

		// Calculate new height with constraints
		const newHeight = Math.max(minHeight, Math.min(maxHeight, node.scrollHeight));
		node.style.height = `${newHeight}px`;
	};

	// Handle input events
	const handleInput = () => {
		resize();
	};

	// Set up initial size and listeners
	resize();
	node.addEventListener('input', handleInput, { passive: true });

	// Also handle value changes from external sources
	const observer = new MutationObserver(() => {
		resize();
	});
	observer.observe(node, { childList: true, characterData: true, subtree: true });

	return {
		update: (newOptions: AutoResizeOptions = {}) => {
			Object.assign(options, newOptions);
			resize();
		},
		destroy: () => {
			node.removeEventListener('input', handleInput);
			observer.disconnect();
		},
		// Expose method for manual resize trigger
		api: {
			resize
		}
	};
}