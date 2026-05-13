<script lang="ts">
	import { getHostMetrics } from '$lib/api';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import HostCard from '$lib/components/HostCard.svelte';
	import type { HostMetrics } from '$lib/types';

	let host = $state<HostMetrics | null>(null);
	let err = $state<string | null>(null);
	let loading = $state(true);
	let retryHost = $state(0);

	let activeSessionsCount = $derived(
		sessionsStore.sessions.filter((s) => s.state !== 'exited').length
	);

	$effect(() => {
		retryHost; // reactive dependency — incrementing restarts effect
		let alive = true;
		const tick = async () => {
			try {
				const m = await getHostMetrics();
				if (!alive) return;
				host = m;
				err = null;
			} catch (e) {
				if (!alive) return;
				err = e instanceof Error ? e.message : 'Failed to load host metrics';
			} finally {
				if (alive) loading = false;
			}
		};
		tick();
		const id = setInterval(tick, 5000);
		return () => {
			alive = false;
			clearInterval(id);
		};
	});
</script>

<div class="fleet-page">
	<div class="page-header">
		<h1>Fleet</h1>
	</div>

	<div class="host-section">
		<HostCard
			metrics={host}
			sessionsCount={activeSessionsCount}
			{loading}
			error={err}
			onRetry={() => retryHost++}
		/>
	</div>

	<div class="empty-remote">
		<span>No remote hosts — single-host mode</span>
	</div>
</div>

<style>
	.fleet-page {
		padding: 20px 24px;
		max-width: 680px;
	}

	.page-header {
		margin-bottom: 20px;
	}

	.page-header h1 {
		margin: 0;
		font-size: 1.2rem;
		font-weight: 700;
		color: var(--text);
	}

	.host-section {
		margin-bottom: 20px;
	}

	.empty-remote {
		padding: 16px 20px;
		background: var(--bg-surface);
		border: 1px dashed var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		font-size: 0.85rem;
		text-align: center;
	}
</style>
