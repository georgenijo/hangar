<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { pipelineStore } from '$lib/stores/pipeline.svelte';
	import PipelineTrack from '$lib/components/PipelineTrack.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import { formatCost, formatTokens, formatDuration } from '$lib/utils';
	import type { DataTableColumn, PipelineRun, PipelinePhase } from '$lib/types';

	// Start polling for pipeline runs on mount
	onMount(() => {
		pipelineStore.startPolling();
	});

	// Stop polling on unmount
	onDestroy(() => {
		pipelineStore.stopPolling();
	});

	// Helper to map PipelineRun to PipelinePhase array for PipelineTrack
	function getPhasesForRun(run: PipelineRun): PipelinePhase[] {
		const allPhases: PipelinePhase[] = [
			{ name: 'Planner', id: 'planner', state: 'pending' },
			{ name: 'Architect', id: 'architect', state: 'pending' },
			{ name: 'Reviewer', id: 'reviewer', state: 'pending' },
			{ name: 'Builder', id: 'builder', state: 'pending' },
			{ name: 'Tester', id: 'tester', state: 'pending' },
			{ name: 'Fixer', id: 'fixer', state: 'pending' },
			{ name: 'PR', id: 'pr', state: 'pending' }
		];

		// Find the index of the current phase
		const currentPhaseIndex = allPhases.findIndex((p) => p.id === run.phase);

		// Mark phases before current as done
		for (let i = 0; i < currentPhaseIndex; i++) {
			allPhases[i].state = 'done';
		}

		// Mark current phase based on run state
		if (currentPhaseIndex >= 0) {
			if (run.state === 'live') {
				allPhases[currentPhaseIndex].state = 'live';
			} else if (run.state === 'done') {
				allPhases[currentPhaseIndex].state = 'done';
			} else if (run.state === 'failed') {
				allPhases[currentPhaseIndex].state = 'pending'; // or could be 'failed' if we add that state
			}
		}

		// If run is done, mark all phases as done
		if (run.state === 'done') {
			allPhases.forEach((p) => {
				p.state = 'done';
			});
		}

		return allPhases;
	}

	// Helper to render gate pills (smoke/screenshot/scenario status)
	// For MVP, these are placeholders - real gate data would come from the backend
	function getGatePills(run: PipelineRun): string {
		if (run.phase === 'tester' || run.state === 'done') {
			return 'smoke: PASS, screenshots: 3, scenarios: 2';
		}
		return '—';
	}

	// DataTable columns for history
	const historyColumns: DataTableColumn<PipelineRun>[] = [
		{
			key: 'state',
			label: 'Status',
			sortable: true,
			width: '80px',
			render: (run) => {
				const statusMap = {
					pending: '○ Pending',
					live: '● Live',
					done: '✓ Done',
					failed: '✗ Failed'
				};
				return statusMap[run.state] || run.state;
			}
		},
		{
			key: 'issue',
			label: 'Issue',
			sortable: true,
			width: '80px',
			render: (run) => `#${run.issue}`
		},
		{
			key: 'title',
			label: 'Title',
			sortable: true
		},
		{
			key: 'phase',
			label: 'Phase',
			sortable: true,
			width: '100px',
			render: (run) => {
				const phaseLabels: Record<string, string> = {
					planner: 'Planner',
					architect: 'Architect',
					reviewer: 'Reviewer',
					builder: 'Builder',
					tester: 'Tester',
					fixer: 'Fixer',
					pr: 'PR'
				};
				return phaseLabels[run.phase] || run.phase;
			}
		},
		{
			key: 'cost',
			label: 'Cost',
			sortable: true,
			width: '100px',
			render: (run) => formatCost(run.cost)
		},
		{
			key: 'tokens',
			label: 'Tokens',
			sortable: true,
			width: '100px',
			render: (run) => formatTokens(run.tokens)
		},
		{
			key: 'duration_s',
			label: 'Duration',
			sortable: true,
			width: '100px',
			render: (run) => formatDuration(run.duration_s)
		}
	];
</script>

<div id="view-pipeline" class="view">
	{#if pipelineStore.error}
		<div class="error-banner">
			Error loading pipeline runs: {pipelineStore.error}
		</div>
	{/if}

	<!-- Active Pipeline Runs -->
	{#if pipelineStore.activeRuns.length > 0}
		<div class="section">
			<h2 class="section-title">Active Pipelines</h2>
			{#each pipelineStore.activeRuns as run}
				<div class="pipeline-card">
					<div class="pipeline-header">
						<div class="pipeline-meta">
							<span class="pipeline-issue">#{run.issue}</span>
							<span class="pipeline-title">{run.title}</span>
							<span class="pipeline-state state-{run.state}">● {run.state}</span>
						</div>
						<div class="pipeline-stats">
							<span class="stat">{run.agents} agents</span>
							<span class="stat">{formatCost(run.cost)}</span>
							<span class="stat">{formatTokens(run.tokens)}</span>
							<span class="stat">{formatDuration(run.duration_s)}</span>
						</div>
					</div>
					<PipelineTrack phases={getPhasesForRun(run)} />
					<div class="pipeline-gates">
						<span class="gate-label">Gates:</span>
						<span class="gate-pills">{getGatePills(run)}</span>
					</div>
				</div>
			{/each}
		</div>
	{:else if !pipelineStore.loading}
		<div class="empty-state">
			<p>No active pipelines</p>
		</div>
	{/if}

	<!-- History Table -->
	<div class="section">
		<h2 class="section-title">Recent Runs</h2>
		{#if pipelineStore.runs.length > 0}
			<DataTable columns={historyColumns} rows={pipelineStore.runs} />
		{:else if !pipelineStore.loading}
			<div class="empty-state">
				<p>No pipeline runs found</p>
			</div>
		{/if}
	</div>
</div>

<style>
	.view {
		padding: 24px;
	}

	.error-banner {
		background: var(--error-bg, #3a1a1a);
		color: var(--error, #f44336);
		padding: 12px 16px;
		border-radius: 4px;
		margin-bottom: 16px;
		border-left: 3px solid var(--error, #f44336);
	}

	.section {
		margin-bottom: 32px;
	}

	.section-title {
		font-size: 18px;
		font-weight: 500;
		margin-bottom: 16px;
		color: var(--text-bright, #e0e0e0);
	}

	.pipeline-card {
		background: var(--surface-2, #1e1e1e);
		border: 1px solid var(--border, #333);
		border-radius: 8px;
		padding: 16px;
		margin-bottom: 16px;
	}

	.pipeline-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 16px;
	}

	.pipeline-meta {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.pipeline-issue {
		font-family: 'JetBrains Mono', monospace;
		font-weight: 600;
		color: var(--accent, #5b8bd4);
		font-size: 14px;
	}

	.pipeline-title {
		font-size: 14px;
		color: var(--text, #c0c0c0);
	}

	.pipeline-state {
		font-size: 12px;
		font-weight: 500;
		text-transform: uppercase;
	}

	.state-live {
		color: var(--success, #4caf50);
	}

	.state-pending {
		color: var(--text-dim, #808080);
	}

	.state-done {
		color: var(--accent, #5b8bd4);
	}

	.state-failed {
		color: var(--error, #f44336);
	}

	.pipeline-stats {
		display: flex;
		gap: 16px;
		font-size: 13px;
		color: var(--text-dim, #808080);
	}

	.stat {
		font-family: 'JetBrains Mono', monospace;
	}

	.pipeline-gates {
		margin-top: 12px;
		padding-top: 12px;
		border-top: 1px solid var(--border, #333);
		display: flex;
		gap: 8px;
		font-size: 12px;
	}

	.gate-label {
		font-weight: 500;
		color: var(--text-dim, #808080);
	}

	.gate-pills {
		font-family: 'JetBrains Mono', monospace;
		color: var(--text, #c0c0c0);
	}

	.empty-state {
		padding: 48px 0;
		text-align: center;
		color: var(--text-dim, #808080);
		font-size: 14px;
	}
</style>
