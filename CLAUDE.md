# Claude Code Session Guide

This document provides context and guidelines for working on the wenxihuang-com project.

## Project Overview

Full-stack web application with:
- **Frontend**: SvelteKit + Vite + Tailwind CSS
- **Backend**: Rust (Axum framework)
- **Database**: PostgreSQL
- **Deployment**: Fly.io

## Important User Preferences

### Git & Deployment
- **DO NOT auto-commit** - Only commit when explicitly requested
- **DO NOT deploy to Fly.io** - Wait for explicit deployment requests

## Styling System

### Theme Architecture
The app uses a **data-theme attribute** system, NOT Tailwind's `dark:` prefix.

```html
<!-- Theme is set on the root element -->
<html data-theme="dark">  <!-- or "light" -->
```

### CSS Custom Properties
Always use these CSS variables defined in [app.css](frontend/src/app.css):

```css
/* Dark Mode - French Brutalism */
--bg-primary: #000000
--text-primary: #ffffff
--text-secondary: #cccccc
--text-tertiary: #808080
--border-subtle: rgba(255, 255, 255, 0.15)
--border-active: rgba(255, 255, 255, 0.4)

/* Light Mode - Japanese Minimalism */
--bg-primary: #ffffff
--text-primary: #000000
--text-secondary: #666666
--text-tertiary: #999999
--border-subtle: rgba(0, 0, 0, 0.08)
--border-active: rgba(0, 0, 0, 0.2)
```

### Theme-Specific Styling Pattern

When creating components, use `:global([data-theme='...'])` for theme-specific styles:

```css
/* Dark mode */
:global([data-theme='dark']) .my-component {
    background: #000000;
    color: #ffffff;
}

/* Light mode */
:global([data-theme='light']) .my-component {
    background: #ffffff;
    color: #000000;
}
```

**DO NOT USE:**
- Tailwind's `dark:` prefix (e.g., `dark:bg-gray-800`) - it won't work
- Hardcoded color classes like `bg-gray-100` - use CSS custom properties instead

### Design Language

**Dual-Theme System:**
- **Dark Mode**: French Brutalism - bold, high contrast, geometric
- **Light Mode**: Japanese Minimalism - clean, spacious, subtle

**Core Principles:**
- Transparent backgrounds with subtle borders
- No rounded corners (or minimal: 0.375rem max)
- No heavy shadows (only subtle ones for dropdowns)
- Uppercase text with generous letter spacing
- Border-only buttons with inverted hover states
- Generous whitespace and breathing room

**Typography:**
```css
/* Dark mode: Bold, assertive */
font-weight: 500-700;
letter-spacing: 0.1-0.15em;

/* Light mode: Light, refined */
font-family: 'Noto Sans JP', sans-serif;
font-weight: 100-200;
letter-spacing: 0.15-0.2em;
```

**Button Pattern:**
```css
.button {
    background: transparent;
    border: 1px solid;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    transition: all 0.3s ease;
}

/* Inverted on hover */
:global([data-theme='dark']) .button:hover {
    background: #ffffff;
    color: #000000;
}

:global([data-theme='light']) .button:hover {
    background: #000000;
    color: #ffffff;
}
```

## Project Structure

```
.
├── backend/
│   ├── src/
│   │   ├── main.rs
│   │   ├── handlers/        # API route handlers
│   │   ├── models/          # Database models
│   │   └── middleware/      # Auth, CORS, etc.
│   └── migrations/          # SQL migrations
│
└── frontend/
    ├── src/
    │   ├── lib/
    │   │   ├── api/         # API client
    │   │   ├── components/  # Reusable components
    │   │   └── stores/      # Svelte stores (auth, etc.)
    │   ├── routes/          # SvelteKit routes
    │   ├── app.css          # Global styles + Tailwind
    │   └── app.html         # HTML template
    └── vite.config.ts
```

## Key Components & Patterns

### Authentication
- Auth state managed by [authStore](frontend/src/lib/stores/auth.ts)
- User data available via `$authStore.user`
- Login redirects handled automatically by the store

### Theme Switching
- Managed by [ThemeToggle](frontend/src/lib/components/ThemeToggle.svelte) component
- Theme persisted to localStorage
- Set immediately on page load to prevent FOUC

### Component Examples
Reference these components for styling patterns:
- [LoginButton.svelte](frontend/src/lib/components/LoginButton.svelte) - Theme-aware positioning & styling
- [UserMenu.svelte](frontend/src/lib/components/UserMenu.svelte) - Dropdown with theme support
- [login/+page.svelte](frontend/src/routes/login/+page.svelte) - Form styling
- [settings/+page.svelte](frontend/src/routes/settings/+page.svelte) - Settings page layout

## Development Workflow

### Frontend
```bash
cd frontend
npm run dev    # Starts Vite dev server
npm run build  # Production build
```

### Backend
```bash
cd backend
cargo run      # Starts Rust server
cargo check    # Type checking
```

### Database
- Local Postgres runs on port 5433 (not default 5432)
- Migrations in `backend/migrations/`

## Common Issues & Solutions

### Tailwind Dark Mode Not Working
❌ **Wrong:** Using `dark:bg-gray-800`
✅ **Correct:** Using `:global([data-theme='dark']) .class { background: #1f1f1f; }`

### Styles Not Updating
- Vite dev server should hot-reload automatically
- Check if dev server is running: `ps aux | grep vite`
- Hard refresh browser (Cmd+Shift+R)

### Padding/Spacing Issues
- Always test hover states to ensure proper internal padding
- Use padding values that create visible gaps: `0.75rem 1rem` minimum
- Check both desktop and mobile breakpoints

## File Naming Conventions

- Routes: `+page.svelte`, `+layout.svelte`, `+server.ts`
- Components: PascalCase (e.g., `UserMenu.svelte`)
- Styles: Scoped within component `<style>` blocks
- API files: camelCase (e.g., `client.ts`)

## Best Practices

1. **Always read existing similar components** before creating new ones
2. **Use CSS custom properties** for colors and theme values
3. **Test both dark and light modes** for every UI change
4. **Keep styling consistent** with the minimalist aesthetic
5. **Use scoped styles** in Svelte components, with `:global()` only for theme selectors
6. **Avoid creating new files** unless necessary - prefer editing existing ones

## Migration Notes

- Migration `003_add_names_to_users.sql` adds `first_name` and `last_name` columns
- User display name logic: `first_name || username`

## Known Configuration

- Frontend dev server: Vite on default port (usually 5173)
- Backend server: Rust/Axum (check main.rs for port)
- Database: PostgreSQL on port 5433
- No Tailwind config file in frontend root (using Tailwind v4 with CSS imports)

---

**Last Updated:** 2025-11-03
