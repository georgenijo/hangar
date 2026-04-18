<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Session } from '$lib/types';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { eventsStore } from '$lib/stores/events.svelte';
	import { sidebarStore } from '$lib/stores/sidebar.svelte';
	import { kindLabel, kindIcon, deleteSession, ApiError } from '$lib/api';
	import { stateColor } from '$lib/utils';
	import InsightsPanel from '$lib/components/InsightsPanel.svelte';
	import TerminalView from '$lib/components/TerminalView.svelte';
	import SandboxDiffView from '$lib/components/SandboxDiffView.svelte';

	let { data }: { data: { session: Session } } = $props();

	let session = $derived(sessionsStore.getSessionById(data.session.id) ?? data.session);
	let insightsCollapsed = $derived(sidebarStore.sessionCollapsed);

	let confirmOpen = $state(false);
	let killing = $state(false);
	let killError = $state<string | null>(null);

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

	function openConfirm() {
		killError = null;
		confirmOpen = true;
	}

	function cancelConfirm() {
		if (killing) return;
		confirmOpen = false;
	}

	async function confirmKill() {
		if (killing) return;
		killing = true;
		killError = null;
		try {
			await deleteSession(session.id);
			// Nudge the sessions store so dashboard does not show a stale tile.
			sessionsStore.removeSession?.(session.id);
			await goto('/');
		} catch (err) {
			killError =
				err instanceof ApiError
					? `${err.status}: ${err.body || err.message}`
					: err instanceof Error
						? err.message
						: 'Unknown error';
			killing = false;
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
		<button
			class="kill-btn"
			type="button"
			title="Kill and remove this session"
			onclick={openConfirm}
		>
			✕ Kill
		</button>
	</div>

	{#if confirmOpen}
		<div
			class="modal-backdrop"
			onclick={cancelConfirm}
			onkeydown={(e) => e.key === 'Escape' && cancelConfirm()}
			role="presentation"
		>
			<div
				class="modal"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
				role="dialog"
				aria-modal="true"
				aria-labelledby="kill-title"
				tabindex={-1}
			>
				<h2 id="kill-title">Kill session <code class="mono">{session.slug}</code>?</h2>
				<p>
					This will terminate the underlying process and permanently remove the session,
					its event history, and its terminal ring buffer. This cannot be undone.
				</p>
				{#if killError}
					<p class="modal-error">Error: {killError}</p>
				{/if}
				<div class="modal-actions">
					<button type="button" class="btn-secondary" onclick={cancelConfirm} disabled={killing}>
						Cancel
					</button>
					<button type="button" class="btn-danger" onclick={confirmKill} disabled={killing}>
						{killing ? 'Killing…' : 'Kill session'}
					</button>
				</div>
			</div>
		</div>
	{/if}

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
			<div class="split" class:insights-collapsed={insightsCollapsed}>
				<div class="main-pane"><TerminalView {session} /></div>
				<aside class="side-pane">
					<div class="side-header">
						<button
							class="collapse-btn"
							onclick={() => sidebarStore.toggleSession()}
							aria-label={insightsCollapsed ? 'Expand insights' : 'Collapse insights'}
							title={insightsCollapsed ? 'Expand (Ctrl+\\)' : 'Collapse (Ctrl+\\)'}
						>
							{insightsCollapsed ? '«' : '»'}
						</button>
					</div>
					{#if !insightsCollapsed}
						<div class="side-body"><InsightsPanel /></div>
					{/if}
				</aside>
			</div>
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
		transition: grid-template-columns 0.25s ease;
	}

	.split.insights-collapsed {
		grid-template-columns: 1fr 36px;
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
		overflow-x: hidden;
		background: var(--surface, var(--bg));
		border-left: 1px solid var(--border);
		display: flex;
		flex-direction: column;
	}

	.side-header {
		display: flex;
		justify-content: flex-start;
		padding: 6px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.split.insights-collapsed .side-header {
		justify-content: center;
	}

	.collapse-btn {
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.9rem;
		line-height: 1;
		padding: 2px 8px;
	}

	.collapse-btn:hover {
		color: var(--text);
		border-color: var(--accent);
	}

	.side-body {
		flex: 1;
		overflow-y: auto;
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

	.kill-btn {
		margin-left: 8px;
		background: transparent;
		color: #f44336;
		border: 1px solid rgba(244, 67, 54, 0.5);
		border-radius: var(--radius);
		padding: 3px 10px;
		font-size: 0.8rem;
		cursor: pointer;
	}

	.kill-btn:hover {
		background: rgba(244, 67, 54, 0.12);
	}

	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.modal {
		background: var(--surface, var(--bg));
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 20px 22px;
		max-width: 440px;
		width: calc(100% - 32px);
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
	}

	.modal h2 {
		font-size: 1rem;
		margin: 0 0 10px 0;
		color: var(--text);
	}

	.modal p {
		font-size: 0.85rem;
		color: var(--text-muted);
		margin: 0 0 14px 0;
		line-height: 1.4;
	}

	.modal-error {
		color: #f44336;
	}

	.modal-actions {
		display: flex;
		gap: 10px;
		justify-content: flex-end;
	}

	.btn-secondary,
	.btn-danger {
		border-radius: var(--radius);
		padding: 6px 14px;
		font-size: 0.85rem;
		cursor: pointer;
		border: 1px solid var(--border);
	}

	.btn-secondary {
		background: transparent;
		color: var(--text);
	}

	.btn-secondary:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.05);
	}

	.btn-danger {
		background: #f44336;
		color: #fff;
		border-color: #f44336;
	}

	.btn-danger:hover:not(:disabled) {
		background: #e53935;
	}

	.btn-secondary:disabled,
	.btn-danger:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
