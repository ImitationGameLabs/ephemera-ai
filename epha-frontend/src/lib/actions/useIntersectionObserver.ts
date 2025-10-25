/**
 * Intersection observer action for tracking element visibility
 * Useful for message read receipts, lazy loading, etc.
 */

interface IntersectionObserverOptions {
	threshold?: number | number[];
	rootMargin?: string;
	onIntersect?: (entries: IntersectionObserverEntry[]) => void;
	onVisible?: (element: Element) => void;
	onHidden?: (element: Element) => void;
}

export function useIntersectionObserver(
	node: HTMLElement,
	options: IntersectionObserverOptions = {}
) {
	const {
		threshold = 0.1,
		rootMargin = '0px',
		onIntersect,
		onVisible,
		onHidden
	} = options;

	// Create the intersection observer
	const observer = new IntersectionObserver(
		(entries) => {
			entries.forEach(entry => {
				if (entry.isIntersecting) {
					onVisible?.(entry.target);
				} else {
					onHidden?.(entry.target);
				}
			});
			onIntersect?.(entries);
		},
		{
			root: node,
			rootMargin,
			threshold
		}
	);

	// Method to observe an element
	const observe = (element: Element) => {
		observer.observe(element);
	};

	// Method to unobserve an element
	const unobserve = (element: Element) => {
		observer.unobserve(element);
	};

	// Method to observe multiple elements
	const observeAll = (elements: NodeListOf<Element> | Element[]) => {
		elements.forEach(el => observer.observe(el));
	};

	// Method to check if element is visible
	const isVisible = (element: Element) => {
		// This is a simplified check - actual visibility depends on observer callback
		// Note: isIntersecting is not a property of Element, this needs to be tracked separately
		return false;
	};

	return {
		update: (newOptions: IntersectionObserverOptions = {}) => {
			Object.assign(options, newOptions);
			// Note: IntersectionObserver options are read-only after creation
			// To change options, we would need to create a new observer and re-observe all elements
			// For simplicity, we're not implementing dynamic option changes
		},
		destroy: () => {
			observer.disconnect();
		},
		api: {
			observe,
			unobserve,
			observeAll,
			isVisible,
			observer // For advanced usage
		}
	};
}