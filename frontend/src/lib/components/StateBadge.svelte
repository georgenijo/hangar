<script lang="ts">
	import { stateColor } from '$lib/utils';
	import type { SessionState } from '$lib/types';

	let { state }: { state: SessionState } = $props();
</script>

<span class="badge" style="background: color-mix(in srgb, {stateColor(state)} 18%, transparent); color: {stateColor(state)};">
	{#if state === 'streaming'}
		<span class="dot pulse"></span>
	{/if}
	{state}
</span>

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 2px 8px;
		border-radius: 12px;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.dot {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: currentColor;
	}

	.pulse {
		animation: pulse 1.2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.3; }
	}
</style>
