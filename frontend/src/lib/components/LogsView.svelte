<script lang="ts">
	import { onMount } from 'svelte';
	import type { LogSource, LogWsMessage } from '$lib/types';
	import { logsStore } from '$lib/stores/logs.svelte';
	import LogFilterChips from './LogFilterChips.svelte';

	let { sources }: { sources: LogSource[] } = $props();

	let container: HTMLElement;
	let sentinel: HTMLElement;
	let laggedBanner = $state('');
	let laggedTimer: ReturnType<typeof setTimeout> | null = null;

	$effect(() => {
		logsStore.setSources(sources);
	});

	function formatTs(ts_us: number): string {
		const d = new Date(ts_us / 1000);
		const h = d.getHours().toString().padStart(2, '0');
		const m = d.getMinutes().toString().padStart(2, '0');
		const s = d.getSeconds().toString().padStart(2, '0');
		const ms = d.getMilliseconds().toString().padStart(3, '0');
		return `${h}:${m}:${s}.${ms}`;
	}

	function levelClass(level: number): string {
		if (level <= 3) return 'level-crit';
		if (level === 4) return 'level-warn';
		if (level === 7) return 'level-debug';
		return '';
	}

	function highlightBody(body: string): string {
		const re = logsStore.searchRegex;
		if (!re) return escapeHtml(body);
		return escapeHtml(body).replace(
			new RegExp(escapeRegexForReplace(re.source), re.flags.replace('g', '') + 'g'),
			(m) => `<mark>${m}</mark>`
		);
	}

	function escapeHtml(s: string): string {
		return s
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;');
	}

	function escapeRegexForReplace(s: string): string {
		return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	}

	onMount(() => {
		logsStore.reset();
		logsStore.setSources(sources);

		let ws: WebSocket | null = null;
		let destroyed = false;
		let reconnectAttempts = 0;
		let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

		function buildWsUrl(): string {
			const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
			const sourceParam =
				logsStore.activeSources.size > 0
					? `&sources=${[...logsStore.activeSources].join(',')}`
					: '';
			return `${proto}//${location.host}/ws/v1/logs?tail=500${sourceParam}`;
		}

		function connect() {
			if (destroyed) return;
			logsStore.setConnected(false);
			ws = new WebSocket(buildWsUrl());

			ws.onopen = () => {
				logsStore.setConnected(true);
				reconnectAttempts = 0;
			};

			ws.onmessage = (e) => {
				let msg: LogWsMessage;
				try {
					msg = JSON.parse(e.data);
				} catch {
					return;
				}
				if (msg.type === 'log') {
					logsStore.addLine(msg);
				} else if (msg.type === 'initial_tail_complete') {
					logsStore.setInitialTailComplete(true);
				} else if (msg.type === 'lagged') {
					const dropped = (msg as { type: 'lagged'; dropped: number }).dropped;
					showLagged(dropped);
				}
			};

			ws.onerror = () => ws?.close();

			ws.onclose = () => {
				if (destroyed) return;
				logsStore.setConnected(false);
				scheduleReconnect();
			};
		}

		function scheduleReconnect() {
			if (destroyed || reconnectAttempts >= 8) return;
			const delay = Math.pow(2, reconnectAttempts) * 1000;
			reconnectAttempts++;
			reconnectTimer = setTimeout(connect, delay);
		}

		function showLagged(dropped: number) {
			laggedBanner = `Dropped ${dropped} lines (slow connection)`;
			if (laggedTimer) clearTimeout(laggedTimer);
			laggedTimer = setTimeout(() => (laggedBanner = ''), 4000);
		}

		connect();

		return () => {
			destroyed = true;
			if (reconnectTimer) clearTimeout(reconnectTimer);
			if (laggedTimer) clearTimeout(laggedTimer);
			ws?.close();
		};
	});

	$effect(() => {
		const lines = logsStore.filteredLines;
		if (logsStore.autoScroll && lines.length > 0 && sentinel) {
			sentinel.scrollIntoView({ block: 'end' });
		}
	});

	function onScroll() {
		if (!container) return;
		const { scrollTop, scrollHeight, clientHeight } = container;
		logsStore.setAutoScroll(scrollHeight - scrollTop - clientHeight < 60);
	}

	function handleToggleSource(name: string) {
		logsStore.toggleSource(name);
	}
</script>

<div class="logs-view">
	<div class="toolbar">
		<LogFilterChips
			sources={logsStore.sources}
			activeSources={logsStore.activeSources}
			onToggle={handleToggleSource}
		/>
		<div class="toolbar-right">
			<input
				class="search-input"
				type="text"
				placeholder="Filter regex…"
				value={logsStore.searchPattern}
				oninput={(e) => logsStore.setSearch((e.target as HTMLInputElement).value)}
			/>
			<button
				class="btn-icon"
				class:active={logsStore.paused}
				onclick={() => logsStore.togglePause()}
				title={logsStore.paused ? 'Resume' : 'Pause'}
			>
				{logsStore.paused ? '▶' : '⏸'}
			</button>
			<button class="btn-icon" onclick={() => logsStore.clear()} title="Clear">🗑</button>
			<label class="autoscroll-label">
				<input
					type="checkbox"
					checked={logsStore.autoScroll}
					onchange={(e) => logsStore.setAutoScroll((e.target as HTMLInputElement).checked)}
				/>
				Autoscroll
			</label>
			<span class="conn-dot" class:connected={logsStore.connected} title={logsStore.connected ? 'Connected' : 'Disconnected'}></span>
		</div>
	</div>

	{#if laggedBanner}
		<div class="lagged-banner">{laggedBanner}</div>
	{/if}

	<div class="log-lines" bind:this={container} onscroll={onScroll}>
		{#each logsStore.filteredLines as line (line.ts_us + line.source + line.body)}
			<div class="log-line {levelClass(line.level)}">
				<span class="ts">{formatTs(line.ts_us)}</span>
				<span class="src">{line.source}</span>
				<!-- eslint-disable-next-line svelte/no-at-html-tags -->
				<span class="body">{@html highlightBody(line.body)}</span>
			</div>
		{/each}
		<div bind:this={sentinel}></div>
	</div>
</div>

<style>
	.logs-view {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--bg);
		overflow: hidden;
	}

	.toolbar {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-surface);
		flex-shrink: 0;
	}

	.toolbar-right {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-left: auto;
	}

	.search-input {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text);
		font-size: 0.82rem;
		padding: 4px 8px;
		width: 180px;
	}

	.search-input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.btn-icon {
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.85rem;
		padding: 4px 8px;
		line-height: 1;
	}

	.btn-icon:hover,
	.btn-icon.active {
		border-color: var(--accent);
		color: var(--text);
	}

	.autoscroll-label {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 0.78rem;
		color: var(--text-muted);
		cursor: pointer;
		user-select: none;
	}

	.conn-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #555;
		flex-shrink: 0;
	}

	.conn-dot.connected {
		background: #4caf50;
	}

	.lagged-banner {
		background: rgba(255, 152, 0, 0.15);
		border-bottom: 1px solid rgba(255, 152, 0, 0.4);
		color: #ff9800;
		font-size: 0.75rem;
		padding: 4px 12px;
		text-align: center;
		flex-shrink: 0;
	}

	.log-lines {
		flex: 1;
		overflow-y: auto;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 4px 0;
	}

	.log-line {
		display: flex;
		gap: 8px;
		padding: 1px 12px;
		line-height: 1.5;
		color: var(--text);
	}

	.log-line:hover {
		background: var(--bg-hover);
	}

	.log-line.level-crit {
		color: #f44336;
	}

	.log-line.level-warn {
		color: #ff9800;
	}

	.log-line.level-debug {
		color: #666;
	}

	.ts {
		color: var(--text-muted);
		flex-shrink: 0;
		font-size: 0.75rem;
	}

	.src {
		color: var(--accent);
		flex-shrink: 0;
		font-size: 0.72rem;
		min-width: 80px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.body {
		flex: 1;
		white-space: pre-wrap;
		word-break: break-all;
	}

	:global(.body mark) {
		background: rgba(255, 220, 0, 0.3);
		color: inherit;
		border-radius: 2px;
	}
</style>
