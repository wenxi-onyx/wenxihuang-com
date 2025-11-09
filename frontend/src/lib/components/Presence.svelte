<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { presenceStore } from '$lib/stores/presence';

	// Subscribe to users from presence store (presenceStore.users is a writable store)
	const users = presenceStore.users;

	let mounted = false;
	let currentPath = $state($page.url.pathname);
	let scrollX = $state(0);
	let scrollY = $state(0);

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

		// Initialize scroll position
		scrollX = window.scrollX;
		scrollY = window.scrollY;

		// Track cursor movements
		const handleMouseMove = (e: MouseEvent) => {
			// Send cursor position as percentage of document (including scroll)
			const documentWidth = Math.max(
				document.documentElement.scrollWidth,
				document.body.scrollWidth,
				1 // Prevent division by zero
			);
			const documentHeight = Math.max(
				document.documentElement.scrollHeight,
				document.body.scrollHeight,
				1 // Prevent division by zero
			);

			// Calculate percentage and round to avoid floating point precision issues
			// Use more decimal places for smoother movement but avoid excessive precision
			const x = Math.round((e.pageX / documentWidth) * 10000) / 100;
			const y = Math.round((e.pageY / documentHeight) * 10000) / 100;

			// Clamp coordinates to 0-100% range
			const clampedX = Math.min(100, Math.max(0, x));
			const clampedY = Math.min(100, Math.max(0, y));
			presenceStore.updateCursor(clampedX, clampedY);
		};

		// Track scroll to update cursor positions in viewport
		const handleScroll = () => {
			scrollX = window.scrollX;
			scrollY = window.scrollY;
		};

		window.addEventListener('mousemove', handleMouseMove, { passive: true });
		window.addEventListener('scroll', handleScroll, { passive: true });

		return () => {
			window.removeEventListener('mousemove', handleMouseMove);
			window.removeEventListener('scroll', handleScroll);
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
				{@const documentWidth = Math.max(
					document.documentElement.scrollWidth,
					document.body.scrollWidth,
					1
				)}
				{@const documentHeight = Math.max(
					document.documentElement.scrollHeight,
					document.body.scrollHeight,
					1
				)}
				{@const absoluteX = (user.cursor.x / 100) * documentWidth}
				{@const absoluteY = (user.cursor.y / 100) * documentHeight}
				{@const viewportX = Math.round(absoluteX - scrollX)}
				{@const viewportY = Math.round(absoluteY - scrollY)}
				{@const isVisible = viewportX >= -20 && viewportX <= window.innerWidth + 20 && viewportY >= -20 && viewportY <= window.innerHeight + 20}
				{#if isVisible}
					<div
						class="cursor"
						style="
							left: {viewportX}px;
							top: {viewportY}px;
							--cursor-color: {user.color};
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
							fill="var(--cursor-color)"
							stroke="var(--bg-primary)"
							stroke-width="1.5"
							stroke-linejoin="round"
						/>
					</svg>
					<div class="username-label">{user.username}</div>
					</div>
				{/if}
			{/if}
		{/each}
	</div>
{/if}

<style>
	/* Using shared styles: animations.css (fadeIn) */

	.presence-container {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		pointer-events: none;
		z-index: 9999;
		overflow: hidden;
	}

	.cursor {
		position: absolute;
		/* Offset by tip position in SVG: path starts at M 2,2 */
		transform: translate(-2px, -2px);
		transition: all 0.1s ease-out;
		pointer-events: none;
		will-change: transform;
		animation: fadeIn 0.2s ease-out;
	}

	.cursor svg {
		filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
	}

	.username-label {
		position: absolute;
		top: 24px;
		left: 8px;
		background: var(--cursor-color);
		color: #ffffff;
		padding: 0.25rem 0.5rem;
		border: 1px solid var(--cursor-color);
		font-size: 0.75rem;
		font-weight: 500;
		white-space: nowrap;
		letter-spacing: 0.02em;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
	}
</style>
