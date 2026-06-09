<script lang="ts">
	interface Props {
		checked?: boolean;
		label?: string;
		onchange?: (checked: boolean) => void;
	}

	let { checked = $bindable(false), label, onchange }: Props = $props();

	function handleChange(event: Event) {
		const target = event.target as HTMLInputElement;
		checked = target.checked;
		if (onchange) {
			onchange(target.checked);
		}
	}
</script>

<div class="toggle-switch-container">
	{#if label}
		<span class="toggle-label">{label}</span>
	{/if}
	<label class="toggle-switch">
		<input
			type="checkbox"
			bind:checked
			onchange={handleChange}
		/>
		<span class="toggle-slider"></span>
	</label>
</div>

<style>
	.toggle-switch-container {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.toggle-label {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-primary);
		opacity: 0.8;
	}

	.toggle-switch {
		position: relative;
		display: inline-block;
		width: 44px;
		height: 24px;
		cursor: pointer;
	}

	.toggle-switch input {
		opacity: 0;
		width: 0;
		height: 0;
	}

	.toggle-slider {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: transparent;
		border: 1px solid var(--border-subtle);
		transition: background-color 0.3s ease, border-color 0.3s ease;
	}

	.toggle-slider::before {
		content: '';
		position: absolute;
		height: 18px;
		width: 14px;
		left: 2px;
		bottom: 2px;
		background-color: var(--text-primary);
		border: 1px solid var(--border-subtle);
		transition: transform 0.3s ease, background-color 0.3s ease, border-color 0.3s ease;
	}

	.toggle-switch input:checked + .toggle-slider {
		background-color: var(--text-primary);
		border-color: var(--text-primary);
	}

	.toggle-switch input:checked + .toggle-slider::before {
		transform: translateX(24px);
		background-color: var(--bg-primary);
		border-color: transparent;
	}

	.toggle-switch:hover .toggle-slider {
		border-color: var(--border-active);
	}
</style>
