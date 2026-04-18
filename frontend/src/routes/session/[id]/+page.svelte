<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Session } from '$lib/types';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { eventsStore } from '$lib/stores/events.svelte';
	import { kindLabel, kindIcon } from '$lib/api';
	import { stateColor } from '$lib/utils';
	import InsightsPanel from '$lib/components/InsightsPanel.svelte';
	import TerminalView from '$lib/components/TerminalView.svelte';
	import SandboxDiffView from '$lib/components/SandboxDiffView.svelte';

	let { data }: { data: { session: Session } } = $props();

	let session = $derived(sessionsStore.getSessionById(data.session.id) ?? data.session);

	$effect(() => {
		const id = data.session.id;
		eventsStore.loadInitialEvents(id).then(() => {
			eventsStore.startEventPolling(id, session.state === 'streaming');
		});
		return () => eventsStore.stopEventPolling();
	});

	function iconChar(icon: string): string {
		switch (icon) {
			case 'terminal': return '⬛';
			case 'bot': return '🤖';
			case 'binary': return '⬜';
			default: return '•';
		}
	}
</script>

<div class="session-page">
	<div class="session-header">
		<button class="back-btn" onclick={() => goto('/')}>← Back</button>
		<span class="slug mono">{session.slug}</span>
		<span class="kind-icon" title={kindLabel(session.kind)}>{iconChar(kindIcon(session.kind))}</span>
		<span
			class="state-badge"
			style="background: {stateColor(session.state)}20; color: {stateColor(session.state)}; border-color: {stateColor(session.state)}40"
		>
			{session.state}
		</span>
		{#if session.state === 'exited'}
			<a class="replay-link" href="/session/{session.id}/replay">▶ Replay</a>
		{/if}
		{#if session.agent_meta?.model}
			<span class="model-name mono">{session.agent_meta.model}</span>
		{/if}
	</div>

	{#if eventsStore.awaitingPermission}
		<div class="permission-banner">
			⚠ Awaiting permission for <code>{eventsStore.awaitingPermission.tool}</code> — respond in terminal
		</div>
	{/if}

	{#if session.sandbox}
		<div class="sandbox-section">
			<div class="sandbox-header">
				<span class="sandbox-badge">🔒 Sandbox: {session.sandbox.state.state}</span>
			</div>
			{#if session.sandbox.state.state === 'running' || session.sandbox.state.state === 'stopped'}
				<SandboxDiffView sessionId={session.id} />
			{:else if session.sandbox.state.state === 'merged'}
				<span class="sandbox-notice">Overlay merged to host</span>
			{:else if session.sandbox.state.state === 'failed'}
				<span class="sandbox-error">Sandbox failed: {session.sandbox.state.message}</span>
			{/if}
		</div>
	{/if}

	<div class="session-body">
		{#key session.id}
			{#if session.kind.type === 'claude_code' || session.kind.type === 'codex'}
				<div class="split">
					<div class="main-pane"><TerminalView {session} /></div>
					<aside class="side-pane"><InsightsPanel /></aside>
				</div>
			{:else}
				<TerminalView {session} />
			{/if}
		{/key}
	</div>
</div>

<style>
	.session-page {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.session-header {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 16px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
		flex-wrap: wrap;
	}

	.back-btn {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.85rem;
		padding: 4px 8px;
	}

	.back-btn:hover {
		color: var(--text);
	}

	.slug {
		font-size: 1rem;
		font-weight: 700;
		color: var(--text);
	}

	.kind-icon {
		font-size: 0.9rem;
	}

	.state-badge {
		display: inline-flex;
		align-items: center;
		padding: 2px 8px;
		border-radius: 12px;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border: 1px solid;
	}

	.replay-link {
		font-size: 0.8rem;
		color: var(--accent);
		text-decoration: none;
		border: 1px solid var(--accent);
		border-radius: var(--radius);
		padding: 2px 8px;
	}

	.replay-link:hover {
		background: rgba(var(--accent-rgb, 100, 180, 255), 0.1);
	}

	.model-name {
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-left: auto;
	}

	.permission-banner {
		padding: 10px 16px;
		background: rgba(255, 152, 0, 0.15);
		border-bottom: 1px solid rgba(255, 152, 0, 0.4);
		color: #ff9800;
		font-size: 0.85rem;
		flex-shrink: 0;
	}

	.permission-banner code {
		font-family: var(--font-mono);
		background: rgba(255, 152, 0, 0.2);
		padding: 1px 5px;
		border-radius: 3px;
	}

	.session-body {
		flex: 1;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.split {
		flex: 1;
		display: grid;
		grid-template-columns: 1fr 280px;
		gap: 1px;
		background: var(--border);
		overflow: hidden;
	}

	.main-pane {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		background: var(--bg);
		min-width: 0;
	}

	.side-pane {
		overflow-y: auto;
		background: var(--surface, var(--bg));
		border-left: 1px solid var(--border);
	}

	.sandbox-section {
		flex-shrink: 0;
		border-bottom: 1px solid var(--border);
		padding: 8px 16px;
	}

	.sandbox-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 6px;
	}

	.sandbox-badge {
		font-size: 0.75rem;
		font-weight: 600;
		color: #7c3aed;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.sandbox-notice {
		font-size: 0.8rem;
		color: #4caf50;
	}

	.sandbox-error {
		font-size: 0.8rem;
		color: #f44336;
	}
</style>
