<script lang="ts">
	import type { Session } from '$lib/types';
	import { eventsStore } from '$lib/stores/events.svelte';
	import { truncate } from '$lib/utils';

	let { session: _session }: { session: Session } = $props();

	let containerEl: HTMLElement;
	let autoScroll = $state(true);
	let expandedThinking = $state<Set<string>>(new Set());
	let expandedTools = $state<Set<string>>(new Set());

	function toggleThinking(key: string) {
		const next = new Set(expandedThinking);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		expandedThinking = next;
	}

	function toggleTool(key: string) {
		const next = new Set(expandedTools);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		expandedTools = next;
	}

	$effect(() => {
		const msgs = eventsStore.chatMessages;
		void msgs;
		if (autoScroll && containerEl) {
			containerEl.scrollTop = containerEl.scrollHeight;
		}
	});

	function onScroll() {
		if (!containerEl) return;
		const atBottom =
			containerEl.scrollHeight - containerEl.scrollTop - containerEl.clientHeight < 50;
		autoScroll = atBottom;
	}

	function relTime(ts: number): string {
		const diff = Math.floor((Date.now() - ts) / 1000);
		if (diff < 60) return `${diff}s ago`;
		const mins = Math.floor(diff / 60);
		if (mins < 60) return `${mins}m ago`;
		return `${Math.floor(mins / 60)}h ago`;
	}
</script>

<div class="chat-container" bind:this={containerEl} onscroll={onScroll}>
	{#if eventsStore.chatMessages.length === 0}
		<p class="empty">No messages yet</p>
	{:else}
		{#each eventsStore.chatMessages as turn (turn.turnId)}
			<div class="turn turn-{turn.role}" class:system={turn.role === 'system'}>
				<div class="turn-header">
					<span class="turn-role">{turn.role}</span>
					{#if !turn.isComplete}
						<span class="streaming-dot"></span>
					{/if}
				</div>

				{#if turn.role === 'system'}
					<details class="system-content">
						<summary>System prompt</summary>
						{#if turn.contentStart}
							<pre class="content-text">{turn.contentStart}</pre>
						{/if}
					</details>
				{:else}
					{#if turn.contentStart}
						<div class="content-text">{@html renderText(turn.contentStart)}</div>
					{/if}

					{#if turn.thinkingBlocks.length > 0}
						{#each turn.thinkingBlocks as tb, i}
							<div class="thinking-block">
								<button
									class="thinking-toggle"
									onclick={() => toggleThinking(`${turn.turnId}-${i}`)}
								>
									💭 Thinking… <span class="char-count">{tb.lenChars} chars</span>
									<span class="chevron">{expandedThinking.has(`${turn.turnId}-${i}`) ? '▲' : '▼'}</span>
								</button>
							</div>
						{/each}
					{/if}

					{#each turn.toolCalls as tc}
						<div class="tool-card">
							<div class="tool-header">
								<span class="tool-name mono">{tc.tool}</span>
								{#if tc.finished}
									<span class="tool-status" class:ok={tc.ok} class:err={!tc.ok}>
										{tc.ok ? '✓' : '✗'}
									</span>
								{:else}
									<span class="tool-running">⟳</span>
								{/if}
								<button
									class="tool-toggle"
									onclick={() => toggleTool(tc.callId)}
								>
									{expandedTools.has(tc.callId) ? '▲' : '▼'}
								</button>
							</div>
							{#if expandedTools.has(tc.callId)}
								<pre class="tool-args">{truncate(tc.argsPreview, 500)}</pre>
								{#if tc.finished && tc.resultPreview}
									<pre class="tool-result" class:ok={tc.ok} class:err={!tc.ok}>{truncate(tc.resultPreview, 500)}</pre>
								{/if}
							{/if}
						</div>
					{/each}
				{/if}
			</div>
		{/each}
	{/if}
</div>

<script module lang="ts">
	function renderText(text: string): string {
		return text
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/`([^`]+)`/g, '<code>$1</code>')
			.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
			.replace(/\n/g, '<br>');
	}
</script>

<style>
	.chat-container {
		flex: 1;
		overflow-y: auto;
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.empty {
		color: var(--text-muted);
		text-align: center;
		padding: 40px;
		font-size: 0.9rem;
	}

	.turn {
		max-width: 80%;
		border-radius: var(--radius);
		padding: 10px 14px;
	}

	.turn-assistant {
		align-self: flex-start;
		background: var(--bg-surface);
		border: 1px solid var(--border);
	}

	.turn-user {
		align-self: flex-end;
		background: rgba(153, 204, 255, 0.1);
		border: 1px solid rgba(153, 204, 255, 0.2);
	}

	.turn-system {
		align-self: stretch;
		max-width: 100%;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		font-style: italic;
	}

	.turn-header {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 6px;
	}

	.turn-role {
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
		font-weight: 600;
	}

	@keyframes blink {
		0%, 100% { opacity: 1; }
		50% { opacity: 0; }
	}

	.streaming-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #4caf50;
		animation: blink 1s ease-in-out infinite;
	}

	.content-text {
		font-size: 0.875rem;
		line-height: 1.5;
		word-break: break-word;
	}

	.content-text :global(code) {
		font-family: var(--font-mono);
		background: var(--bg);
		padding: 1px 4px;
		border-radius: 3px;
		font-size: 0.8em;
	}

	.thinking-block {
		margin-top: 6px;
	}

	.thinking-toggle {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.8rem;
		padding: 4px 10px;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.thinking-toggle:hover {
		color: var(--text);
	}

	.char-count {
		color: var(--text-muted);
		font-size: 0.75em;
	}

	.chevron {
		font-size: 0.65em;
	}

	.tool-card {
		margin-top: 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.tool-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		background: var(--bg);
	}

	.tool-name {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text);
		flex: 1;
	}

	.tool-status {
		font-size: 0.8rem;
	}

	.tool-status.ok { color: #4caf50; }
	.tool-status.err { color: #f44336; }

	.tool-running {
		color: var(--text-muted);
		font-size: 0.85rem;
		animation: spin 1s linear infinite;
		display: inline-block;
	}

	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}

	.tool-toggle {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.65rem;
		padding: 2px 6px;
	}

	.tool-args,
	.tool-result {
		margin: 0;
		padding: 8px 10px;
		font-family: var(--font-mono);
		font-size: 0.78rem;
		overflow-x: auto;
		white-space: pre-wrap;
		word-break: break-all;
		background: var(--bg);
		border-top: 1px solid var(--border);
		color: var(--text-muted);
	}

	.tool-result.ok { color: #4caf50; }
	.tool-result.err { color: #f44336; }

	.system-content summary {
		cursor: pointer;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	pre.content-text {
		font-family: var(--font-mono);
		font-size: 0.78rem;
		white-space: pre-wrap;
		margin: 0;
	}
</style>
