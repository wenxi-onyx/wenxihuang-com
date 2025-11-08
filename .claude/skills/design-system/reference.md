# Design System - Full Component Reference

Complete code examples for all component patterns. Use this as a copy-paste reference when implementing components.

## Buttons

### Primary CTA (Outlined)

```svelte
<button class="card-cta">ENTER</button>

<style>
  .card-cta {
    padding: 0.75rem 2rem;
    font-size: 0.875rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    background: transparent;
    cursor: pointer;
    transition: all 0.3s ease;
    border: none;
  }

  :global([data-theme='dark']) .card-cta {
    color: #ffffff;
    border: 1px solid #ffffff;
  }

  :global([data-theme='dark']) .card-cta:hover {
    background: #ffffff;
    color: #000000;
  }

  :global([data-theme='light']) .card-cta {
    color: #000000;
    border: 0.5px solid #000000;
    font-weight: 200;
  }

  :global([data-theme='light']) .card-cta:hover {
    background: #000000;
    color: #ffffff;
  }
</style>
```

### Text Button / Link

```svelte
<button class="nav-link-btn">ADD MATCH</button>

<style>
  .nav-link-btn {
    font-size: 0.875rem;
    font-weight: 300;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: inherit;
    opacity: 0.7;
    transition: opacity 0.2s ease;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: inherit;
  }

  .nav-link-btn:hover {
    opacity: 1;
  }
</style>
```

## Cards

### Feature Card

```svelte
<a href="/app" class="card active">
  <h3 class="card-title">FEATURE NAME</h3>
  <button class="card-cta">ENTER</button>
</a>

<style>
  .card {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    height: 420px;
    padding: 3rem 2rem;
    text-decoration: none;
    transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    background: var(--bg-primary);
    opacity: 0;
    animation: fadeInCard 0.4s ease-out forwards;
  }

  :global([data-theme='dark']) .card {
    background: #000000;
    color: #ffffff;
    border: 1px solid rgba(255, 255, 255, 0.3);
  }

  :global([data-theme='light']) .card {
    background: #ffffff;
    color: #000000;
    border: 1px solid rgba(0, 0, 0, 0.15);
  }

  :global([data-theme='dark']) .card.active {
    border-width: 2px;
    border-color: rgba(255, 255, 255, 0.35);
  }

  :global([data-theme='light']) .card.active {
    border-width: 1px;
    border-color: rgba(0, 0, 0, 0.2);
  }

  .card.active:hover {
    transform: translateY(-4px);
  }

  :global([data-theme='dark']) .card.active:hover {
    border-color: rgba(255, 255, 255, 0.4);
    box-shadow: 0 8px 24px rgba(255, 255, 255, 0.1);
  }

  :global([data-theme='light']) .card.active:hover {
    border-color: rgba(0, 0, 0, 0.2);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.04);
  }

  .card-title {
    font-size: 1.5rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    text-align: center;
    margin: 0;
  }

  :global([data-theme='dark']) .card-title {
    font-weight: 400;
  }

  :global([data-theme='light']) .card-title {
    font-weight: 200;
  }

  @keyframes fadeInCard {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .card {
      animation: none;
      opacity: 1;
    }
    .card.active:hover {
      transform: none;
    }
  }
</style>
```

## Forms

### Text Input

```svelte
<input
  type="text"
  placeholder="Search players..."
  bind:value={searchQuery}
  class="search-input"
/>

<style>
  .search-input {
    width: 400px;
    padding: 0.75rem 1rem;
    font-size: 1rem;
    font-family: inherit;
    background: transparent;
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
    outline: none;
    transition: border-color 0.2s ease;
  }

  .search-input:focus {
    border-color: var(--border-active);
  }

  .search-input::placeholder {
    color: var(--text-primary);
    opacity: 0.3;
  }

  @media (max-width: 768px) {
    .search-input {
      width: 100%;
    }
  }
</style>
```

### Select Dropdown

```svelte
<select bind:value={selectedSeasonId} class="season-selector">
  {#each seasons as season}
    <option value={season.id}>
      {season.name}{season.is_active ? ' (Active)' : ''}
    </option>
  {/each}
</select>

<style>
  .season-selector {
    width: 400px;
    padding: 0.75rem 1rem;
    font-size: 0.875rem;
    font-family: inherit;
    font-weight: 300;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background: transparent;
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
    outline: none;
    transition: border-color 0.2s ease;
    cursor: pointer;
  }

  :global([data-theme='dark']) .season-selector {
    font-weight: 400;
  }

  :global([data-theme='light']) .season-selector {
    font-weight: 200;
  }

  .season-selector:focus {
    border-color: var(--border-active);
  }

  .season-selector option {
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary);
  }

  :global([data-theme='dark']) .season-selector option {
    background: #1a1a1a;
  }

  @media (max-width: 768px) {
    .season-selector {
      width: 100%;
    }
  }
</style>
```

## Tables

### Leaderboard Table

```svelte
<div class="table-wrapper">
  <table class="leaderboard-table">
    <thead>
      <tr>
        <th>Rank</th>
        <th>Player</th>
        <th>ELO Rating</th>
        <th>Games</th>
        <th>Wins</th>
        <th>Losses</th>
        <th>Win Rate</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      {#each players as player, index}
        <tr>
          <td class="rank-cell">#{index + 1}</td>
          <td class="name-cell">
            <a href="/players/{player.id}" class="player-name-link">
              {player.name}
            </a>
          </td>
          <td class="elo-cell">{player.current_elo.toFixed(1)}</td>
          <td>{player.games_played}</td>
          <td>{player.wins}</td>
          <td>{player.losses}</td>
          <td>{getWinRate(player)}%</td>
          <td>
            <a href="/players/{player.id}" class="btn-view">View History</a>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .table-wrapper {
    overflow-x: auto;
    border: 1px solid var(--border-subtle);
    background: transparent;
  }

  .leaderboard-table {
    width: 100%;
    border-collapse: collapse;
  }

  .leaderboard-table thead {
    background: transparent;
    border-bottom: 1px solid var(--border-subtle);
  }

  .leaderboard-table th {
    padding: 1rem;
    text-align: left;
    font-weight: 300;
    color: var(--text-primary);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    opacity: 0.8;
  }

  :global([data-theme='dark']) .leaderboard-table th {
    font-weight: 500;
  }

  :global([data-theme='light']) .leaderboard-table th {
    font-weight: 200;
  }

  .leaderboard-table tbody tr {
    border-bottom: 1px solid var(--border-subtle);
    transition: opacity 0.15s;
  }

  .leaderboard-table tbody tr:hover {
    opacity: 0.8;
  }

  .leaderboard-table td {
    padding: 1rem;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .player-name-link {
    text-decoration: none;
    color: inherit;
    transition: opacity 0.2s ease;
  }

  .player-name-link:hover {
    opacity: 0.6;
    text-decoration: underline;
    text-decoration-thickness: 0.5px;
  }

  .btn-view {
    display: inline-block;
    padding: 0.5rem 0;
    font-size: 0.75rem;
    font-weight: 300;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-primary);
    text-decoration: underline;
    text-decoration-thickness: 0.5px;
    border: none;
    background: transparent;
    transition: opacity 0.3s ease;
  }

  :global([data-theme='light']) .btn-view {
    font-weight: 200;
  }

  .btn-view:hover {
    opacity: 0.6;
  }

  @media (max-width: 768px) {
    .leaderboard-table {
      font-size: 0.8rem;
    }

    .leaderboard-table th,
    .leaderboard-table td {
      padding: 0.75rem 0.5rem;
    }
  }
</style>
```

## Toasts

### Toast Notification System

```svelte
<script lang="ts" context="module">
  import { writable } from 'svelte/store';

  export type ToastType = 'success' | 'error';
  export type Toast = { id: number; message: string; type: ToastType };

  let toastId = 0;
  const toasts = writable<Toast[]>([]);
  const timeouts = new Map<number, ReturnType<typeof setTimeout>>();

  export function showToast(message: string, type: ToastType = 'success') {
    const id = toastId++;
    toasts.update(t => [...t, { id, message, type }]);

    const timeout = setTimeout(() => {
      toasts.update(t => t.filter(toast => toast.id !== id));
      timeouts.delete(id);
    }, 4000);

    timeouts.set(id, timeout);
  }
</script>

<div class="toast-container">
  {#each $toasts as toast (toast.id)}
    <div class="toast toast-{toast.type}">
      {toast.message}
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 2rem;
    left: 2rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    z-index: 9999;
  }

  .toast {
    padding: 0.875rem 1.25rem;
    font-size: 0.875rem;
    font-weight: 300;
    letter-spacing: 0.05em;
    border: 1px solid;
    color: var(--text-primary);
    min-width: 250px;
    max-width: 400px;
    animation: slideIn 0.3s ease-out;
  }

  @keyframes slideIn {
    from {
      transform: translateX(-100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  .toast-success {
    opacity: 0.95;
  }

  :global([data-theme='dark']) .toast-success {
    border-color: rgba(255, 255, 255, 0.3);
    background: rgba(255, 255, 255, 0.1);
  }

  :global([data-theme='light']) .toast-success {
    border-color: rgba(0, 0, 0, 0.2);
    background: rgba(0, 0, 0, 0.05);
    font-weight: 200;
  }

  .toast-error {
    opacity: 0.95;
  }

  :global([data-theme='dark']) .toast-error {
    border-color: rgba(255, 255, 255, 0.3);
    background: rgba(255, 255, 255, 0.1);
  }

  :global([data-theme='light']) .toast-error {
    border-color: rgba(0, 0, 0, 0.2);
    background: rgba(0, 0, 0, 0.05);
    font-weight: 200;
  }

  @media (max-width: 768px) {
    .toast-container {
      bottom: 1.5rem;
      left: 1.5rem;
      right: 1.5rem;
    }

    .toast {
      min-width: auto;
    }
  }
</style>
```

## Dropdowns

### Dropdown Menu

```svelte
<script>
  let showDropdown = $state(false);
</script>

<div class="dropdown">
  <button
    class="dropdown-toggle"
    onclick={() => showDropdown = !showDropdown}
    onblur={() => setTimeout(() => showDropdown = false, 200)}
  >
    MANAGE ▾
  </button>
  {#if showDropdown}
    <div class="dropdown-menu">
      <a href="/admin/seasons" class="dropdown-item">Seasons</a>
      <a href="/admin/elo" class="dropdown-item">Elo Algorithms</a>
      <a href="/admin/players" class="dropdown-item">Players</a>
    </div>
  {/if}
</div>

<style>
  .dropdown {
    position: relative;
  }

  .dropdown-toggle {
    font-size: 0.875rem;
    font-weight: 300;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: inherit;
    opacity: 0.7;
    transition: opacity 0.2s ease;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: inherit;
  }

  .dropdown-toggle:hover {
    opacity: 1;
  }

  .dropdown-menu {
    position: absolute;
    top: calc(100% + 0.5rem);
    right: 0;
    background: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    min-width: 200px;
    z-index: 1000;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .dropdown-item {
    display: block;
    padding: 0.75rem 1rem;
    font-size: 0.875rem;
    font-weight: 300;
    letter-spacing: 0.05em;
    text-decoration: none;
    color: var(--text-primary);
    opacity: 0.8;
    transition: all 0.2s ease;
    border-bottom: 1px solid var(--border-subtle);
  }

  .dropdown-item:last-child {
    border-bottom: none;
  }

  .dropdown-item:hover {
    opacity: 1;
    background: var(--border-subtle);
  }
</style>
```

## Page Layouts

### Page Header with Navigation

```svelte
<header class="page-header">
  <h1>Table Tennis Leaderboard</h1>
  <nav class="nav-links">
    <a href="/matches">MATCH HISTORY</a>
    <a href="/admin">ADMIN</a>
    <button class="nav-link-btn" onclick={() => window.history.back()}>BACK</button>
  </nav>
</header>

<style>
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 3rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .page-header h1 {
    font-size: clamp(1.5rem, 4vw, 2.5rem);
    font-weight: 300;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    margin: 0;
    color: var(--text-primary);
  }

  .nav-links {
    display: flex;
    align-items: center;
    gap: 2rem;
  }

  .nav-links a {
    font-size: 0.875rem;
    font-weight: 300;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    text-decoration: none;
    color: inherit;
    opacity: 0.7;
    transition: opacity 0.2s ease;
  }

  .nav-links a:hover {
    opacity: 1;
  }

  @media (max-width: 768px) {
    .page-header {
      flex-direction: column;
      gap: 1rem;
      align-items: flex-start;
    }
  }
</style>
```

### Landing Page Grid

```svelte
<main class="landing-page">
  <section class="features">
    <div class="cards-container">
      <FeatureCard title="PROJECT 1" status="active" href="/project1" />
      <FeatureCard title="PROJECT 2" status="active" href="/project2" />
      <FeatureCard title="WIP" status="inactive" />
    </div>
  </section>
</main>

<style>
  .landing-page {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    padding: 4rem 2rem 2rem;
    gap: 3.5rem;
  }

  .features {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .cards-container {
    display: grid;
    grid-template-columns: repeat(3, 320px);
    gap: 3rem;
  }

  @media (max-width: 1200px) {
    .cards-container {
      grid-template-columns: repeat(3, 240px);
      gap: 2rem;
    }
  }

  @media (max-width: 768px) {
    .landing-page {
      padding: 3.5rem 1rem 2rem;
      gap: 2rem;
    }

    .cards-container {
      display: flex;
      flex-direction: column;
      gap: 2rem;
      width: 100%;
      max-width: 400px;
    }
  }
</style>
```

## Animations

### Fade In

```css
@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

/* Usage */
.element {
  opacity: 0;
  animation: fadeIn 2s ease-out forwards;
}

.element-delayed {
  opacity: 0;
  animation: fadeIn 1s ease-out 0.5s forwards;
}
```

### Fade In Card

```css
@keyframes fadeInCard {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Usage */
.card {
  opacity: 0;
  animation: fadeInCard 0.4s ease-out forwards;
  animation-delay: 0.3s; /* Optional stagger */
}
```

### Slide In (Toast)

```css
@keyframes slideIn {
  from {
    transform: translateX(-100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

/* Usage */
.toast {
  animation: slideIn 0.3s ease-out;
}
```

### Accessibility - Always Include

```css
@media (prefers-reduced-motion: reduce) {
  .animated-element {
    animation: none;
    opacity: 1;
    transform: none;
  }
}
```
