<script lang="ts">
	import { goto } from '$app/navigation';
	import { getHostMetrics, getDailyCosts, getPipelineRuns } from '$lib/api';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { isPipelineInFlight, formatTokens, formatIdleTime } from '$lib/utils';
	import KpiCard from '$lib/components/KpiCard.svelte';
	import SpendBarChart from '$lib/components/SpendBarChart.svelte';
	import RingGauge from '$lib/components/RingGauge.svelte';
	import StateBadge from '$lib/components/StateBadge.svelte';
	import type { HostMetrics, DailyCost, PipelineRun } from '$lib/types';

	// --- state ---
	let host = $state<HostMetrics | null>(null);
	let hostErr = $state<string | null>(null);
	let loadingHost = $state(true);

	let days = $state<DailyCost[]>([]);
	let costsErr = $state<string | null>(null);
	let loadingCosts = $state(true);

	let runs = $state<PipelineRun[]>([]);
	let pipelinesErr = $state<string | null>(null);
	let loadingPipelines = $state(true);

	// --- derived ---
	let activeSessions = $derived(sessionsStore.sessions.filter((s) => s.state !== 'exited'));
	let today = $derived(days[0] ?? null);
	let last7 = $derived(days.slice(0, 7));
	let pipelinesRunning = $derived(runs.filter((r) => isPipelineInFlight(r.state)).length);

	// --- retry fns (exposed so error banners can call them) ---
	let retryHost = $state(0);
	let retryCosts = $state(0);
	let retryPipelines = $state(0);

	// --- host metrics effect (5s) ---
	$effect(() => {
		retryHost; // reactive dependency
		let alive = true;
		const tick = async () => {
			try {
				const m = await getHostMetrics();
				if (!alive) return;
				host = m;
				hostErr = null;
			} catch (e) {
				if (!alive) return;
				hostErr = e instanceof Error ? e.message : 'Failed to load host metrics';
			} finally {
				if (alive) loadingHost = false;
			}
		};
		tick();
		const id = setInterval(tick, 5000);
		return () => { alive = false; clearInterval(id); };
	});

	// --- costs effect (60s) ---
	$effect(() => {
		retryCosts; // reactive dependency
		let alive = true;
		const tick = async () => {
			try {
				const c = await getDailyCosts();
				if (!alive) return;
				days = c.days;
				costsErr = null;
			} catch (e) {
				if (!alive) return;
				costsErr = e instanceof Error ? e.message : 'Failed to load costs';
			} finally {
				if (alive) loadingCosts = false;
			}
		};
		tick();
		const id = setInterval(tick, 60_000);
		return () => { alive = false; clearInterval(id); };
	});

	// --- pipeline runs effect (5s) ---
	$effect(() => {
		retryPipelines; // reactive dependency
		let alive = true;
		const tick = async () => {
			try {
				const r = await getPipelineRuns();
				if (!alive) return;
				runs = r.runs;
				pipelinesErr = null;
			} catch (e) {
				if (!alive) return;
				pipelinesErr = e instanceof Error ? e.message : 'Failed to load pipeline runs';
			} finally {
				if (alive) loadingPipelines = false;
			}
		};
		tick();
		const id = setInterval(tick, 5000);
		return () => { alive = false; clearInterval(id); };
	});
</script>

<div class="command-page">
	<!-- KPI row -->
	<div class="kpi-row">
		<KpiCard label="Sessions Active" value={activeSessions.length} />
		<KpiCard
			label="Cost Today"
			value={today ? '$' + today.usd.toFixed(2) : (loadingCosts ? '…' : '—')}
		/>
		<KpiCard
			label="Tokens Today"
			value={today ? formatTokens(today.tokens) : (loadingCosts ? '…' : '—')}
		/>
		<KpiCard
			label="Pipelines Running"
			value={loadingPipelines ? '…' : (pipelinesErr && runs.length === 0 ? '—' : pipelinesRunning)}
		/>
	</div>

	<!-- Charts + fleet row -->
	<div class="mid-row">
		<!-- Spend bar chart -->
		<div class="section-card chart-card">
			<div class="section-header">
				<h2>Spend, last 7 days</h2>
			</div>
			{#if costsErr}
				<div class="error-banner">
					{costsErr}
					<button class="retry-btn" onclick={() => retryCosts++}>Retry</button>
				</div>
			{/if}
			<div class="chart-wrap">
				<SpendBarChart days={last7} />
			</div>
		</div>

		<!-- Fleet gauges -->
		<div class="section-card gauges-card">
			<div class="section-header">
				<h2>Host</h2>
				{#if host}
					<span class="host-name">{host.hostname}</span>
				{/if}
			</div>
			{#if hostErr}
				<div class="error-banner">
					{hostErr}
					<button class="retry-btn" onclick={() => retryHost++}>Retry</button>
				</div>
			{/if}
			<div class="gauges-row">
				<RingGauge pct={host?.cpu_pct ?? 0} label="CPU" size={80} />
				<RingGauge pct={host?.ram_pct ?? 0} label="RAM" size={80} />
				<RingGauge pct={host?.disk_pct ?? 0} label="Disk" size={80} />
			</div>
		</div>
	</div>

	<!-- Active sessions list -->
	<div class="section-card">
		<div class="section-header">
			<h2>Active sessions</h2>
			<span class="count-badge">{activeSessions.length}</span>
		</div>
		{#if activeSessions.length === 0}
			<div class="empty-state">No active sessions</div>
		{:else}
			<div class="session-list">
				{#each activeSessions as s (s.id)}
					<button
						class="session-row"
						onclick={() => goto('/session/' + s.id)}
					>
						<span class="session-slug">{s.slug}</span>
						<StateBadge state={s.state} />
						<span class="session-tokens">
							{formatTokens(s.agent_meta?.tokens_used ?? 0)} tok
						</span>
						<span class="cost-pill">
							{s.agent_meta?.cost_dollars != null
								? '$' + s.agent_meta.cost_dollars.toFixed(2)
								: '—'}
						</span>
					</button>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Pipelines in flight -->
	<div class="section-card">
		<div class="section-header">
			<h2>Pipeline runs</h2>
			<span class="count-badge">{runs.length}</span>
		</div>
		{#if pipelinesErr}
			<div class="error-banner">
				{pipelinesErr}
				<button class="retry-btn" onclick={() => retryPipelines++}>Retry</button>
			</div>
		{/if}
		{#if !loadingPipelines && runs.length === 0 && !pipelinesErr}
			<div class="empty-state">No pipeline runs</div>
		{:else}
			<div class="pipeline-list">
				{#each runs as run (`${run.issue}-${run.phase}-${run.started_at}`)}
					<div class="pipeline-row">
						<span class="pipeline-issue">#{run.issue}</span>
						<span class="pipeline-state state-{run.state}">{run.state}</span>
						<span class="pipeline-phase">{run.phase}</span>
						<span class="pipeline-host">{run.host}</span>
						<span class="pipeline-age">
							{run.started_at ? formatIdleTime(new Date(run.started_at).getTime()) + ' ago' : '—'}
						</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.command-page {
		padding: 20px 24px;
		display: flex;
		flex-direction: column;
		gap: 16px;
		max-width: 960px;
	}

	/* KPI row */
	.kpi-row {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 12px;
	}

	/* Mid row: chart + gauges */
	.mid-row {
		display: grid;
		grid-template-columns: 2fr 1fr;
		gap: 16px;
	}

	/* Section cards */
	.section-card {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 16px;
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 12px;
	}

	.section-header h2 {
		margin: 0;
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.count-badge {
		font-size: 0.75rem;
		font-family: var(--font-mono);
		background: var(--bg-hover);
		color: var(--text-muted);
		padding: 1px 7px;
		border-radius: 10px;
	}

	.host-name {
		font-size: 0.82rem;
		font-family: var(--font-mono);
		color: var(--text-muted);
		margin-left: auto;
	}

	/* Chart */
	.chart-card {
		display: flex;
		flex-direction: column;
	}

	.chart-wrap {
		flex: 1;
	}

	/* Gauges */
	.gauges-card {
		display: flex;
		flex-direction: column;
	}

	.gauges-row {
		display: flex;
		gap: 16px;
		justify-content: center;
		padding: 8px 0;
		flex: 1;
		align-items: center;
	}

	/* Session list */
	.session-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.session-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--radius);
		background: none;
		border: none;
		cursor: pointer;
		text-align: left;
		width: 100%;
		color: var(--text);
		transition: background 0.1s;
	}

	.session-row:hover {
		background: var(--bg-hover);
	}

	.session-slug {
		flex: 1;
		font-family: var(--font-mono);
		font-size: 0.85rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}

	.session-tokens {
		font-size: 0.75rem;
		font-family: var(--font-mono);
		color: var(--text-muted);
		white-space: nowrap;
	}

	.cost-pill {
		font-size: 0.75rem;
		font-family: var(--font-mono);
		color: var(--accent);
		white-space: nowrap;
		min-width: 3.5rem;
		text-align: right;
	}

	/* Pipeline list */
	.pipeline-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.pipeline-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 7px 10px;
		border-radius: var(--radius);
		font-size: 0.82rem;
	}

	.pipeline-row:hover {
		background: var(--bg-hover);
	}

	.pipeline-issue {
		font-family: var(--font-mono);
		font-weight: 600;
		color: var(--accent);
		min-width: 3rem;
	}

	.pipeline-state {
		font-size: 0.72rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding: 2px 8px;
		border-radius: 10px;
		background: var(--bg-hover);
		color: var(--text-muted);
	}

	.pipeline-state.state-running,
	.pipeline-state.state-in_progress {
		background: color-mix(in srgb, #4caf50 18%, transparent);
		color: #4caf50;
	}

	.pipeline-state.state-completed {
		background: color-mix(in srgb, #5b8bd4 18%, transparent);
		color: #5b8bd4;
	}

	.pipeline-state.state-failed {
		background: color-mix(in srgb, #f44336 18%, transparent);
		color: #f44336;
	}

	.pipeline-phase {
		font-family: var(--font-mono);
		font-size: 0.78rem;
		color: var(--text-muted);
		flex: 1;
	}

	.pipeline-host {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.pipeline-age {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--text-muted);
		min-width: 2.5rem;
		text-align: right;
	}

	/* Empty + error states */
	.empty-state {
		padding: 20px;
		text-align: center;
		color: var(--text-muted);
		font-size: 0.85rem;
	}

	.error-banner {
		display: flex;
		align-items: center;
		gap: 10px;
		background: color-mix(in srgb, #f44336 12%, transparent);
		border: 1px solid color-mix(in srgb, #f44336 30%, transparent);
		border-radius: var(--radius);
		padding: 6px 12px;
		font-size: 0.82rem;
		color: #f44336;
		margin-bottom: 10px;
	}

	.retry-btn {
		background: none;
		border: 1px solid #f44336;
		border-radius: var(--radius);
		color: #f44336;
		cursor: pointer;
		font-size: 0.78rem;
		padding: 2px 8px;
		margin-left: auto;
		flex-shrink: 0;
	}

	.retry-btn:hover {
		background: color-mix(in srgb, #f44336 15%, transparent);
	}

	@media (max-width: 700px) {
		.kpi-row {
			grid-template-columns: 1fr 1fr;
		}
		.mid-row {
			grid-template-columns: 1fr;
		}
	}
</style>
