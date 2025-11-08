---
name: design-system
description: Dual-theme design system (French Brutalism dark + Japanese Minimalism light) for public-facing pages. Reference this when building new pages/components to ensure consistency. Admin pages may vary.
---

# Design System

**Dual-theme system:** French Brutalism (Dark) + Japanese Minimalism (Light)

## Quick Reference

```
Framework:     Tailwind CSS v4 (@tailwindcss/forms, @tailwindcss/typography)
Fonts:         System stack | Noto Sans JP (100-400) | Cormorant Garamond (300-400)
Font Weights:  Dark: 400-700 | Light: 100-300
Breakpoint:    768px (mobile) | 1200px (desktop compact)
Transitions:   0.2s-0.3s ease | Cards: 0.4s cubic-bezier(0.4, 0, 0.2, 1)
Text Style:    Uppercase + letter-spacing: 0.1em (all UI elements)
Opacity:       Default: 0.7 | Hover: 1.0 | Labels: 0.8 | Placeholders: 0.3
```

## Color System

```css
/* Dark Mode - French Brutalism */
--bg-primary: #000000
--text-primary: #ffffff
--text-secondary: #cccccc
--text-tertiary: #808080
--border-subtle: rgba(255, 255, 255, 0.15)
--border-active: rgba(255, 255, 255, 0.4)

/* Light Mode - Japanese Minimalism */
[data-theme='light']
--bg-primary: #ffffff
--text-primary: #000000
--text-secondary: #666666
--text-tertiary: #999999
--border-subtle: rgba(0, 0, 0, 0.08)
--border-active: rgba(0, 0, 0, 0.2)
```

**Accent Colors (Charts/Data Viz):**
Blue `rgba(59, 130, 246, 0.9)` | Purple `rgba(168, 85, 247, 0.9)` | Green `rgba(34, 197, 94, 0.9)` | Yellow `rgba(234, 179, 8, 0.9)` | Red `rgba(239, 68, 68, 0.9)` | Pink `rgba(236, 72, 153, 0.9)`

## Typography

| Element | Size | Letter Spacing |
|---------|------|----------------|
| Hero names | `clamp(2.5rem, 6vw, 5rem)` | 0.15em (dark) / 0.2em (light) |
| Page headers | `clamp(1.5rem, 4vw, 2.5rem)` | 0.1em |
| Body text | 1rem | 0.05em |
| Inputs/buttons | 0.875rem | 0.1em |
| Nav/labels | 0.75rem | 0.1em |
| Badges | 0.625rem | 0.05em |
| Chart text | 9-11px | - |

**Font Weights (Critical):**
```css
/* Dark mode: bolder for contrast */
:global([data-theme='dark']) .element { font-weight: 400-700; }

/* Light mode: lighter for elegance */
:global([data-theme='light']) .element { font-weight: 100-300; }
```

**Text Transform:** Uppercase for all headings, buttons, nav, table headers

## Spacing Scale

| Type | Value | Usage |
|------|-------|-------|
| **Page Padding** |
| Large pages | `6rem 2rem 4rem 2rem` | Top breathing room |
| Landing | `4rem 2rem 2rem` | Standard pages |
| Mobile | `3.5rem 1rem 2rem` | Small screens |
| **Component Gaps** |
| Sections | 3.5rem | Large spacing |
| Grids | 3rem | Card layouts |
| Nav/controls | 2rem | Medium spacing |
| Compact | 1rem | Tight elements |
| Toasts | 0.75rem | Stacked items |
| **Element Padding** |
| Table cells | 1rem | Standard |
| Inputs | `0.75rem 1rem` | Form fields |
| Buttons | `0.5rem 2rem` | CTAs |
| Toasts | `0.875rem 1.25rem` | Notifications |

**Container Widths:** `1200px` (main) | `400px` (inputs/mobile cards) | `320px` (desktop cards) | `240px` (compact cards)

## Component Patterns

### Buttons

**Primary CTA (Outlined):**
- Transparent background, uppercase, `letter-spacing: 0.1em`
- Dark: `border: 1px solid #fff`, hover inverts to white bg/black text
- Light: `border: 0.5px solid #000, font-weight: 200`, hover inverts
- `padding: 0.75rem 2rem; transition: all 0.3s ease;`

**Text Button/Link:**
- No background/border, `opacity: 0.7`, hover to `1.0`
- `font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.1em;`

### Cards

**Feature Card:**
- `height: 420px; padding: 3rem 2rem; background: var(--bg-primary);`
- Dark: `border: 1px solid rgba(255,255,255,0.3)`, hover `0.4 + shadow`
- Light: `border: 1px solid rgba(0,0,0,0.15)`, hover `0.2 + shadow`
- Hover: `transform: translateY(-4px); transition: all 0.4s cubic-bezier(0.4,0,0.2,1);`

### Forms

**Text Input:**
- `background: transparent; border: 1px solid var(--border-subtle);`
- Focus: `border-color: var(--border-active);`
- Placeholder: `opacity: 0.3;`

**Select:**
- Same as input + `text-transform: uppercase; letter-spacing: 0.05em;`
- Dark options: `background: #1a1a1a;`

### Tables

**Leaderboard:**
- Wrapper: `border: 1px solid var(--border-subtle); overflow-x: auto;`
- Headers: `font-size: 0.75rem; text-transform: uppercase; opacity: 0.8;`
- Rows: `border-bottom: 1px solid var(--border-subtle);`
- Hover: `opacity: 0.8; transition: opacity 0.15s;`

### Toasts

- Fixed `bottom: 2rem; left: 2rem; z-index: 9999;`
- `padding: 0.875rem 1.25rem; border: 1px solid;`
- Dark: `border-color: rgba(255,255,255,0.3); background: rgba(255,255,255,0.1);`
- Light: `border-color: rgba(0,0,0,0.2); background: rgba(0,0,0,0.05); font-weight: 200;`
- Animation: `slideIn 0.3s ease-out` (translateX from -100%)

### Dropdowns

- `position: absolute; background: var(--bg-primary); border: 1px solid var(--border-subtle);`
- Items: `padding: 0.75rem 1rem; opacity: 0.8;`, hover `opacity: 1 + background: var(--border-subtle);`

## Layout Patterns

**Grid (Feature Cards):**
```css
display: grid;
grid-template-columns: repeat(3, 320px);  /* 240px @ <1200px */
gap: 3rem;  /* 2rem @ <1200px */
```

**Flex (Page Header):**
```css
display: flex;
justify-content: space-between;
align-items: center;
margin-bottom: 3rem;
padding-bottom: 1rem;
border-bottom: 1px solid rgba(255,255,255,0.1);
```

**Centered Content:**
```css
width: 100%;
display: flex;
flex-direction: column;
align-items: center;
justify-content: flex-start;
```

**Breakpoints:**
- `@media (max-width: 768px)`: Stack vertically, reduce padding (1rem), hide non-critical table columns
- `@media (max-width: 1200px)`: Compact grid (240px cards, 2rem gap)

## Animations

**Standard Pattern:**
```css
@keyframes fadeInCard {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

/* Usage */
animation: fadeInCard 0.4s ease-out forwards;
animation-delay: 0.5s;  /* Optional */
```

**Common Animations:**
- `fadeIn`: opacity 0→1 (hero: 2s, links: 1s with 0.5s delay)
- `fadeInCard`: opacity + translateY for cards (0.4s)
- `slideIn`: translateX(-100%) for toasts (0.3s)

**Always include:**
```css
@media (prefers-reduced-motion: reduce) {
  .element { animation: none; opacity: 1; transform: none; }
}
```

## Theme Implementation

**HTML attribute:** `<html data-theme="dark">` or `"light"`

**Persistence (app.html):**
```javascript
(function() {
  const theme = localStorage.getItem('theme') || 'dark';
  document.documentElement.setAttribute('data-theme', theme);
})();
```

**Styling pattern:**
```css
:global([data-theme='dark']) .element { /* dark styles */ }
:global([data-theme='light']) .element { /* light styles */ }
```

## Design Philosophy

### French Brutalism (Dark Mode)
Pure black (#000) backgrounds, white text, heavy weights (400-700), stark borders (0.3-0.4 opacity), system fonts, geometric precision

### Japanese Minimalism (Light Mode)
Pure white (#fff) backgrounds, black text, light weights (100-300), subtle borders (0.08-0.2 opacity), Noto Sans JP influence, generous negative space

### Shared Principles
Uppercase typography with generous letter-spacing, smooth transitions (0.2-0.4s), mobile-first responsive, reduced motion support, transparent backgrounds with borders

## Critical Patterns

1. **Theme-specific font weights** - Dark: 400-700 | Light: 100-300
2. **Always use CSS variables** - `var(--bg-primary)`, `var(--text-primary)`, etc.
3. **Uppercase UI text** - `text-transform: uppercase; letter-spacing: 0.1em;`
4. **Theme-specific borders** - Use `:global([data-theme='...'])` pattern
5. **Rem-based spacing** - Multiples of 0.5rem or defined patterns above
6. **Mobile breakpoint: 768px** - Stack layouts, reduce padding, adjust fonts
7. **Interactive opacity** - Default: 0.7 | Hover: 1.0
8. **Standard transitions** - `0.2s ease` or `0.3s ease` (cards: `0.4s cubic-bezier`)
9. **Reduced motion support** - Always include `@media (prefers-reduced-motion: reduce)`
10. **Transparent backgrounds** - Most components use `background: transparent` with borders

---

**See [reference.md](reference.md) for full component code examples.**
