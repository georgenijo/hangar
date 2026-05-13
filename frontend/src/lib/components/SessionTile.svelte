<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Session } from '$lib/types';
	import { normalizeLabels, kindLabel, kindIcon, deleteSession } from '$lib/api';
	import { formatIdleTime, formatTokens, truncate, stateColor } from '$lib/utils';
	import { sessionsStore } from '$lib/stores/sessions.svelte';

	let { session }: { session: Session } = $props();

	let idleTime = $state('');
	let removing = $state(false);
	let removeError = $state('');

	$effect(() => {
		idleTime = formatIdleTime(session.last_activity_at);
		const id = setInterval(() => {
			idleTime = formatIdleTime(session.last_activity_at);
		}, 1000);
		return () => clearInterval(id);
	});

	let labels = $derived(normalizeLabels(session.labels));
	let visibleLabels = $derived(labels.slice(0, 3));
	let extraCount = $derived(labels.length - visibleLabels.length);

	function iconChar(icon: string): string {
		switch (icon) {
			case 'terminal':
				return '⬛';
			case 'bot':
				return '🤖';
			case 'binary':
				return '⬜';
			default:
				return '•';
		}
	}

	async function handleRemove(e: MouseEvent) {
		e.stopPropagation();
		const isLive = session.state !== 'exited';
		if (isLive && !confirm(`Remove session "${session.slug}"? This will kill the process.`)) return;
		removing = true;
		removeError = '';
		try {
			await deleteSession(session.id);
			sessionsStore.removeSession(session.id);
		} catch (err) {
			removeError = err instanceof Error ? err.message : 'Remove failed';
			removing = false;
		}
	}
</script>

<div
	class="tile"
	role="button"
	tabindex="0"
	onclick={(e) => { if (!(e.target as HTMLElement).closest('.remove-btn')) goto(`/session/${session.id}`); }}
	onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') goto(`/session/${session.id}`); }}
>
	<div class="tile-header">
		<span class="slug mono">{session.slug}</span>
		<span class="kind-icon" title={kindLabel(session.kind)}>{iconChar(kindIcon(session.kind))}</span>
		<button
			class="remove-btn"
			class:removing
			onclick={handleRemove}
			title="Remove session"
			disabled={removing}
			aria-label="Remove session"
		>✕</button>
		<span class="state-badge" style="background: {stateColor(session.state)}20; color: {stateColor(session.state)}; border-color: {stateColor(session.state)}40">
			<span class="state-dot" style="background: {stateColor(session.state)}" class:pulse={session.state === 'streaming'}></span>
			{session.state}
		</span>
		{#if session.sandbox}
			<span class="sandbox-badge">
				🔒 {session.sandbox.state.state}
			</span>
		{/if}
	</div>

	<div class="tile-body">
		<div class="meta-row">
			<span class="meta-label">Model</span>
			<span class="meta-value mono">{session.agent_meta?.model ?? '—'}</span>
		</div>
		<div class="meta-row">
			<span class="meta-label">Tokens</span>
			<span class="meta-value mono">{formatTokens(session.agent_meta?.tokens_used ?? 0)}</span>
		</div>
		<div class="meta-row">
			<span class="meta-label">Last tool</span>
			<span class="meta-value mono">{session.agent_meta?.last_tool_call ? truncate(session.agent_meta.last_tool_call, 30) : '—'}</span>
		</div>
		<div class="meta-row">
			<span class="meta-label">Idle</span>
			<span class="meta-value mono">{idleTime}</span>
		</div>
	</div>

	{#if labels.length > 0}
		<div class="tile-footer">
			{#each visibleLabels as label}
				<span class="label-chip">
					{label.value ? `${label.key}=${label.value}` : label.key}
				</span>
			{/each}
			{#if extraCount > 0}
				<span class="label-extra">+{extraCount}</span>
			{/if}
		</div>
	{/if}
	{#if removeError}
		<div class="remove-error">{removeError}</div>
	{/if}
</div>

<style>
	.tile {
		display: flex;
		flex-direction: column;
		gap: 10px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 14px;
		cursor: pointer;
		text-align: left;
		width: 100%;
		color: var(--text);
		transition: border-color 0.15s, transform 0.1s;
		min-width: 280px;
		outline: none;
	}

	.tile:focus-visible {
		border-color: var(--accent);
	}

	.tile:hover {
		border-color: var(--accent);
		transform: translateY(-1px);
	}

	.tile-header {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.slug {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--text);
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.kind-icon {
		font-size: 0.85rem;
	}

	.state-badge {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 2px 8px;
		border-radius: 12px;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border: 1px solid;
	}

	.state-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.3; }
	}

	.state-dot.pulse {
		animation: pulse 1.2s ease-in-out infinite;
	}

	.tile-body {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.meta-row {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		font-size: 0.8rem;
	}

	.meta-label {
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.meta-value {
		color: var(--text);
		text-align: right;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}

	.tile-footer {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}

	.label-chip {
		font-family: var(--font-mono);
		font-size: 0.7rem;
		padding: 2px 6px;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 10px;
		color: var(--text-muted);
	}

	.label-extra {
		font-size: 0.7rem;
		color: var(--text-muted);
		padding: 2px 4px;
	}

	.sandbox-badge {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 2px 8px;
		border-radius: 12px;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border: 1px solid;
		background: #7c3aed20;
		color: #7c3aed;
		border-color: #7c3aed40;
	}

	.remove-btn {
		margin-left: auto;
		background: none;
		border: 1px solid transparent;
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.75rem;
		line-height: 1;
		padding: 2px 6px;
		flex-shrink: 0;
		opacity: 0;
		transition: opacity 0.15s, color 0.1s, border-color 0.1s;
	}

	.tile:hover .remove-btn {
		opacity: 1;
	}

	.remove-btn:hover {
		color: #f44336;
		border-color: rgba(244, 67, 54, 0.4);
	}

	.remove-btn.removing {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.remove-error {
		font-size: 0.75rem;
		color: #f44336;
		padding: 2px 0;
	}
</style>
