# Frontend Technology Stack & Conventions

## Technology Stack

Our frontend is built with the following modern web technologies:

- **Svelte 5**: The latest version of Svelte with runes and enhanced reactivity
- **SvelteKit**: Full-stack web framework for building web applications
- **TypeScript**: Type-safe JavaScript for better development experience and code maintainability
- **Skeleton 3**: Component library and utility framework built on top of Tailwind CSS
- **Tailwind CSS 4**: Utility-first CSS framework for rapid UI development

## Color System & Theming

### Color Palette Management

We utilize **Skeleton 3's color system** for systematic color hierarchy design. This approach provides:

- **Consistent color naming**: Semantic color variables that follow a logical hierarchy
- **Theme management**: Centralized color definitions that can be easily modified
- **Light/Dark mode support**: Built-in support for automatic theme switching
- **Scalable design system**: Easy to extend and maintain as the application grows

### Implementation Guidelines

1. **Use Skeleton 3 color system**: Always prefer Skeleton's color system over hardcoded colors
2. **Semantic naming**: Use semantic color names (e.g., `bg-surface-200-800`, `text-primary-400-600`, `border-secondary-50-950`) rather than specific hex values
3. **Theme-aware components**: Design components that automatically adapt to light/dark themes
4. **Consistent spacing and typography**: Follow Skeleton's design system for spacing and typography

### Benefits

- **Systematic color hierarchy**: Organized color structure with clear relationships between colors
- **Easy theme management**: Simplified process for managing multiple themes
- **Better maintainability**: Changes to color schemes can be made centrally
- **Enhanced user experience**: Seamless light/dark mode switching
- **Design consistency**: Uniform appearance across the entire application