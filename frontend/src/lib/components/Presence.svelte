<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { presenceStore } from '$lib/stores/presence';

	// Subscribe to users from presence store (presenceStore.users is a writable store)
	const users = presenceStore.users;

	let mounted = false;
	let currentPath = $state($page.url.pathname);

	// Watch for page changes and update presence
	$effect(() => {
		const pagePath = $page.url.pathname;
		if (mounted && pagePath !== currentPath) {
			currentPath = pagePath;
			presenceStore.joinPage(pagePath);
		}
	});

	onMount(() => {
		mounted = true;

		// Join current page
		presenceStore.joinPage($page.url.pathname);

		// Track cursor movements
		const handleMouseMove = (e: MouseEvent) => {
			// Send cursor position as percentage of viewport
			const x = (e.clientX / window.innerWidth) * 100;
			const y = (e.clientY / window.innerHeight) * 100;
			presenceStore.updateCursor(x, y);
		};

		window.addEventListener('mousemove', handleMouseMove, { passive: true });

		return () => {
			window.removeEventListener('mousemove', handleMouseMove);
		};
	});

	onDestroy(() => {
		mounted = false;
		presenceStore.leavePage();
	});
</script>

{#if $users && $users.length > 0}
	<div class="presence-container">
		{#each $users as user (user.user_id)}
			{#if user.cursor}
				<div
					class="cursor"
					style="
						left: {user.cursor.x}%;
						top: {user.cursor.y}%;
					"
				>
					<svg
						width="20"
						height="20"
						viewBox="0 0 20 20"
						fill="none"
						xmlns="http://www.w3.org/2000/svg"
					>
						<path
							d="M 2,2 L 2,16 L 6,12 L 9,18 L 11,17 L 8,11 L 14,11 Z"
							fill="var(--text-primary)"
							stroke="var(--bg-primary)"
							stroke-width="1.5"
							stroke-linejoin="round"
						/>
					</svg>
					<div class="username-label">{user.username}</div>
				</div>
			{/if}
		{/each}
	</div>
{/if}

<style>
	.presence-container {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		pointer-events: none;
		z-index: 9999;
	}

	.cursor {
		position: absolute;
		transform: translate(-3px, -3px);
		transition: all 0.1s ease-out;
		pointer-events: none;
		will-change: transform;
	}

	.cursor svg {
		filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
	}

	.username-label {
		position: absolute;
		top: 24px;
		left: 8px;
		background: var(--text-primary);
		color: var(--bg-primary);
		padding: 0.25rem 0.5rem;
		border: 1px solid var(--text-primary);
		font-size: 0.75rem;
		font-weight: 400;
		white-space: nowrap;
		letter-spacing: 0.02em;
	}

	/* Smooth animations */
	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: scale(0.8);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}

	.cursor {
		animation: fadeIn 0.2s ease-out;
	}
</style>
