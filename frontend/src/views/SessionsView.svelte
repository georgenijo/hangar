<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import { currentView } from '$lib/router.svelte';
	import { formatCost, formatTokens } from '$lib/utils';
	import type { DataTableColumn, Session } from '$lib/types';

	onMount(() => sessionsStore.startPolling());
	onDestroy(() => sessionsStore.stopPolling());

	// Filter state
	type FilterType = 'all' | 'active' | 'idle' | 'exited';
	let activeFilter: FilterType = $state('all');

	// Filter sessions based on selected filter
	const filteredSessions = $derived.by(() => {
		const sessions = sessionsStore.filteredSessions;
		switch (activeFilter) {
			case 'active':
				return sessions.filter((s) => s.state === 'streaming');
			case 'idle':
				return sessions.filter((s) => s.state === 'idle' || s.state === 'awaiting');
			case 'exited':
				return sessions.filter((s) => s.state === 'exited');
			default:
				return sessions;
		}
	});

	const columns: DataTableColumn<Session>[] = [
		{
			key: 'state',
			label: 'Status',
			sortable: true,
			width: '80px',
			render: (s) => {
				switch (s.state) {
					case 'streaming':
						return '●';
					case 'idle':
						return '○';
					case 'awaiting':
						return '◐';
					case 'error':
						return '✕';
					case 'exited':
						return '–';
					default:
						return '◌';
				}
			}
		},
		{ key: 'slug', label: 'Slug', sortable: true },
		{
			key: 'kind',
			label: 'Agent',
			sortable: true,
			render: (s) => s.kind.type
		},
		{ key: 'node_id', label: 'Host', sortable: true },
		{
			key: 'ctx_pct',
			label: 'ctx%',
			sortable: false,
			render: (s) => {
				if (s.agent_meta) {
					// Extract context percentage from agent_meta if available
					return '–';
				}
				return '–';
			}
		},
		{
			key: 'tokens',
			label: 'Tokens',
			sortable: true,
			render: (s) => (s.agent_meta ? formatTokens(s.agent_meta.tokens_used) : '0')
		},
		{
			key: 'cost',
			label: 'Cost',
			sortable: false,
			render: (_s) => {
				// Cost would need to be computed from events, not directly on Session
				return '–';
			}
		}
	];

	function handleRowClick(session: Session) {
		// Store selected session ID and navigate to session-detail
		sessionStorage.setItem('selectedSessionId', session.id);
		currentView.navigate('session-detail');
	}
</script>

<div id="view-sessions" class="view">
	<div class="filter-bar">
		<button
			class="filter-chip"
			class:active={activeFilter === 'all'}
			onclick={() => (activeFilter = 'all')}
		>
			All
		</button>
		<button
			class="filter-chip"
			class:active={activeFilter === 'active'}
			onclick={() => (activeFilter = 'active')}
		>
			Active
		</button>
		<button
			class="filter-chip"
			class:active={activeFilter === 'idle'}
			onclick={() => (activeFilter = 'idle')}
		>
			Idle
		</button>
		<button
			class="filter-chip"
			class:active={activeFilter === 'exited'}
			onclick={() => (activeFilter = 'exited')}
		>
			Exited
		</button>
	</div>

	<DataTable columns={columns} rows={filteredSessions} onRowClick={handleRowClick} />
</div>

<style>
	.view {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.filter-bar {
		display: flex;
		gap: 8px;
	}

	.filter-chip {
		padding: 6px 12px;
		background: var(--surface-2, #2a2a2a);
		border: 1px solid var(--surface-3, #3a3a3a);
		border-radius: 4px;
		color: var(--text-dim, #999);
		cursor: pointer;
		font-size: 13px;
		transition: all 0.2s;
	}

	.filter-chip:hover {
		background: var(--surface-3, #3a3a3a);
		color: var(--text, #e0e0e0);
	}

	.filter-chip.active {
		background: var(--accent, #5b8bd4);
		color: var(--text, #ffffff);
		border-color: var(--accent, #5b8bd4);
	}
</style>
