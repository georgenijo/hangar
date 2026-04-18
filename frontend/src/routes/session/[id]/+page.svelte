<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Session } from '$lib/types';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { eventsStore } from '$lib/stores/events.svelte';
	import { kindLabel, kindIcon } from '$lib/api';
	import { stateColor } from '$lib/utils';
	import ChatView from '$lib/components/ChatView.svelte';
	import InsightsPanel from '$lib/components/InsightsPanel.svelte';
	import TerminalView from '$lib/components/TerminalView.svelte';

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
		{#if session.agent_meta?.model}
			<span class="model-name mono">{session.agent_meta.model}</span>
		{/if}
	</div>

	{#if eventsStore.awaitingPermission}
		<div class="permission-banner">
			⚠ Awaiting permission for <code>{eventsStore.awaitingPermission.tool}</code> — respond in terminal
		</div>
	{/if}

	{#if eventsStore.contextUsage}
		<div class="context-bar">
			<div
				class="context-fill"
				style="width: {eventsStore.contextUsage.pctUsed}%; background: {eventsStore.contextUsage.pctUsed > 80 ? '#f44336' : eventsStore.contextUsage.pctUsed > 60 ? '#ff9800' : 'var(--accent)'}"
			></div>
			<span class="context-label">{Math.round(eventsStore.contextUsage.pctUsed)}% context</span>
		</div>
	{/if}

	{#if session.kind.type === 'claude_code' || session.kind.type === 'codex'}
		<InsightsPanel />
	{/if}

	<div class="session-body">
		{#if session.kind.type === 'claude_code' || session.kind.type === 'codex'}
			<div class="split">
				<div class="split-pane chat-pane"><ChatView {session} /></div>
				<div class="split-pane term-pane"><TerminalView {session} /></div>
			</div>
		{:else}
			<TerminalView {session} />
		{/if}
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

	.context-bar {
		position: relative;
		height: 4px;
		background: var(--border);
		flex-shrink: 0;
	}

	.context-fill {
		height: 100%;
		transition: width 0.5s;
	}

	.context-label {
		position: absolute;
		right: 8px;
		top: 6px;
		font-size: 0.7rem;
		color: var(--text-muted);
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
		grid-template-columns: 1fr 1fr;
		gap: 1px;
		background: var(--border);
		overflow: hidden;
	}

	.split-pane {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		background: var(--bg);
		min-width: 0;
	}
</style>
