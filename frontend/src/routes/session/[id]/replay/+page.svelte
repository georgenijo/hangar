<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import type { Session, StoredEvent } from '$lib/types';
	import { getSessionOutput, kindLabel, kindIcon } from '$lib/api';
	import { ApiError } from '$lib/api';

	let { data }: { data: { session: Session; events: StoredEvent[] } } = $props();

	let session = $derived(data.session);
	let allEvents = $derived(data.events);

	let outputEvents = $derived(
		allEvents.filter((e) => e.event.type === 'output_appended') as (StoredEvent & {
			event: { type: 'output_appended'; offset: number; len: number };
		})[]
	);

	let resizeEvents = $derived(
		allEvents.filter((e) => e.event.type === 'resized') as (StoredEvent & {
			event: { type: 'resized'; cols: number; rows: number };
		})[]
	);

	let turnBoundaries = $derived(
		allEvents.filter(
			(e) =>
				e.event.type === 'agent_event' &&
				e.event.event?.type === 'turn_started'
		) as (StoredEvent & {
			event: { type: 'agent_event'; id: string; event: { type: 'turn_started'; turn_id: number } };
		})[]
	);

	let startTs = $derived(allEvents[0]?.ts ?? 0);
	let endTs = $derived(allEvents[allEvents.length - 1]?.ts ?? 0);

	let playing = $state(false);
	let speed = $state(1);
	let currentEventIndex = $state(0);
	let seeking = $state(false);
	let truncated = $state(false);
	let currentTs = $derived(outputEvents[currentEventIndex]?.ts ?? startTs);

	let container: HTMLElement;
	let term: import('@xterm/xterm').Terminal | null = null;
	let rafId: number | null = null;
	let lastWallTime = 0;
	let lastEventTs = 0;

	const SPEEDS = [0.5, 1, 2, 4, 8];

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

	function formatElapsed(ts: number): string {
		if (!startTs) return '0:00';
		const diff = Math.max(0, ts - startTs);
		const s = Math.floor(diff / 1000);
		const m = Math.floor(s / 60);
		const ss = s % 60;
		return `${m}:${ss.toString().padStart(2, '0')}`;
	}

	function resolveSizeAt(ts: number): { cols: number; rows: number } | null {
		let last: { cols: number; rows: number } | null = null;
		for (const e of resizeEvents) {
			if (e.ts <= ts) last = { cols: e.event.cols, rows: e.event.rows };
			else break;
		}
		return last;
	}

	async function writeOutputEvent(
		ev: (typeof outputEvents)[number]
	): Promise<boolean> {
		try {
			const { data } = await getSessionOutput(session.id, {
				offset: ev.event.offset,
				len: ev.event.len
			});
			if (data.byteLength > 0) {
				term?.write(new Uint8Array(data));
			}
			return true;
		} catch (e) {
			if (e instanceof ApiError && e.status === 410) {
				truncated = true;
				return false;
			}
			return false;
		}
	}

	async function seekTo(ts: number) {
		if (!term) return;
		seeking = true;
		playing = false;
		if (rafId !== null) {
			cancelAnimationFrame(rafId);
			rafId = null;
		}

		term.reset();
		truncated = false;

		const targetEvents = outputEvents.filter((e) => e.ts <= ts);
		const BATCH = 50;

		for (let i = 0; i < targetEvents.length; i += BATCH) {
			const batch = targetEvents.slice(i, i + BATCH);

			// Apply resize events that fall before this batch's first event
			const batchTs = batch[0]?.ts ?? 0;
			const size = resolveSizeAt(batchTs);
			if (size && term) {
				try {
					term.resize(size.cols, size.rows);
				} catch {
					// ignore invalid resize
				}
			}

			for (const ev of batch) {
				const ok = await writeOutputEvent(ev);
				if (!ok && truncated) break;
			}

			if (truncated) break;

			// Yield to main thread between batches
			await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
		}

		currentEventIndex = outputEvents.findIndex((e) => e.ts >= ts);
		if (currentEventIndex === -1) currentEventIndex = outputEvents.length;

		seeking = false;
	}

	function play() {
		if (currentEventIndex >= outputEvents.length) {
			currentEventIndex = 0;
			term?.reset();
		}
		playing = true;
		lastWallTime = performance.now();
		lastEventTs = outputEvents[currentEventIndex]?.ts ?? startTs;
		scheduleFrame();
	}

	function pause() {
		playing = false;
		if (rafId !== null) {
			cancelAnimationFrame(rafId);
			rafId = null;
		}
	}

	function scheduleFrame() {
		rafId = requestAnimationFrame(onFrame);
	}

	function onFrame() {
		if (!playing) return;

		const now = performance.now();
		const wallElapsed = now - lastWallTime;
		const eventElapsed = wallElapsed * speed;

		const nextIndex = currentEventIndex;
		if (nextIndex >= outputEvents.length) {
			playing = false;
			return;
		}

		const nextEv = outputEvents[nextIndex];
		const timeSinceLastEvent = nextEv.ts - lastEventTs;

		if (eventElapsed >= timeSinceLastEvent) {
			lastWallTime = now - (eventElapsed - timeSinceLastEvent) / speed;
			lastEventTs = nextEv.ts;

			// Apply any resize events before this output event
			const size = resolveSizeAt(nextEv.ts);
			if (size && term) {
				try {
					term.resize(size.cols, size.rows);
				} catch {
					// ignore
				}
			}

			writeOutputEvent(nextEv).then(() => {
				currentEventIndex = nextIndex + 1;
				if (playing) scheduleFrame();
			});
			return;
		}

		scheduleFrame();
	}

	onMount(() => {
		let destroyed = false;

		async function init() {
			const { Terminal } = await import('@xterm/xterm');
			const { FitAddon } = await import('@xterm/addon-fit');

			if (destroyed) return;

			const link = document.createElement('link');
			link.rel = 'stylesheet';
			link.href = new URL('@xterm/xterm/css/xterm.css', import.meta.url).href;
			document.head.appendChild(link);

			term = new Terminal({
				disableStdin: true,
				theme: {
					background: '#0e0e10',
					foreground: '#eeeeee',
					cursor: '#99ccff',
					selectionBackground: '#2a2a30'
				},
				fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
				fontSize: 13,
				cursorBlink: false
			});

			const fitAddon = new FitAddon();
			term.loadAddon(fitAddon);
			term.open(container);
			fitAddon.fit();

			const resizeObserver = new ResizeObserver(() => {
				fitAddon.fit();
			});
			resizeObserver.observe(container);

			return () => {
				resizeObserver.disconnect();
			};
		}

		let cleanup: (() => void) | undefined;
		init().then((fn) => {
			cleanup = fn;
		});

		return () => {
			destroyed = true;
			if (rafId !== null) cancelAnimationFrame(rafId);
			term?.dispose();
			cleanup?.();
		};
	});
</script>

<div class="replay-page">
	<div class="replay-header">
		<button class="back-btn" onclick={() => goto(`/session/${session.id}`)}>← Back</button>
		<span class="slug mono">{session.slug}</span>
		<span class="kind-icon" title={kindLabel(session.kind)}>{iconChar(kindIcon(session.kind))}</span>
		<span class="replay-label">Replay</span>
		<span class="ts-display mono">{formatElapsed(currentTs)} / {formatElapsed(endTs)}</span>
	</div>

	{#if truncated}
		<div class="truncated-banner">
			⚠ History truncated — early output was overwritten by ring buffer
		</div>
	{/if}

	{#if seeking}
		<div class="seeking-overlay">
			<span>Seeking…</span>
		</div>
	{/if}

	<div class="terminal-wrapper">
		<div class="terminal-container" bind:this={container}></div>
	</div>

	<div class="controls">
		<button
			class="play-btn"
			onclick={() => (playing ? pause() : play())}
			disabled={seeking}
			type="button"
		>
			{playing ? '⏸' : '▶'}
		</button>

		<div class="speed-controls">
			{#each SPEEDS as s (s)}
				<button
					class="speed-btn"
					class:active={speed === s}
					onclick={() => (speed = s)}
					type="button"
				>
					{s}×
				</button>
			{/each}
		</div>

		<input
			class="scrubber"
			type="range"
			min={startTs}
			max={endTs}
			value={currentTs}
			disabled={seeking}
			oninput={(e) => {
				const ts = parseInt((e.target as HTMLInputElement).value, 10);
				seekTo(ts);
			}}
		/>

		{#if turnBoundaries.length > 0}
			<select
				class="turn-select"
				onchange={(e) => {
					const ts = parseInt((e.target as HTMLSelectElement).value, 10);
					if (!isNaN(ts)) seekTo(ts);
				}}
			>
				<option value="">Jump to turn…</option>
				{#each turnBoundaries as tb (tb.id)}
					<option value={tb.ts}>
						Turn {tb.event.event.turn_id} — {formatElapsed(tb.ts)}
					</option>
				{/each}
			</select>
		{/if}
	</div>
</div>

<style>
	.replay-page {
		display: flex;
		flex-direction: column;
		height: 100%;
		position: relative;
	}

	.replay-header {
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

	.replay-label {
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--accent);
		border: 1px solid var(--accent);
		border-radius: 10px;
		padding: 1px 8px;
	}

	.ts-display {
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-left: auto;
	}

	.truncated-banner {
		padding: 8px 16px;
		background: rgba(255, 152, 0, 0.15);
		border-bottom: 1px solid rgba(255, 152, 0, 0.4);
		color: #ff9800;
		font-size: 0.82rem;
		flex-shrink: 0;
	}

	.seeking-overlay {
		position: absolute;
		inset: 0;
		background: rgba(14, 14, 16, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 20;
		font-size: 1.1rem;
		color: var(--text-muted);
	}

	.terminal-wrapper {
		flex: 1;
		background: #0e0e10;
		overflow: hidden;
	}

	.terminal-container {
		width: 100%;
		height: 100%;
		padding: 4px;
	}

	.controls {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 16px;
		border-top: 1px solid var(--border);
		flex-shrink: 0;
		flex-wrap: wrap;
	}

	.play-btn {
		background: var(--accent);
		border: none;
		border-radius: var(--radius);
		color: #000;
		cursor: pointer;
		font-size: 1rem;
		padding: 4px 12px;
		min-width: 40px;
	}

	.play-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.speed-controls {
		display: flex;
		gap: 4px;
	}

	.speed-btn {
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.75rem;
		padding: 2px 6px;
	}

	.speed-btn.active {
		border-color: var(--accent);
		color: var(--accent);
	}

	.scrubber {
		flex: 1;
		min-width: 100px;
		accent-color: var(--accent);
	}

	.turn-select {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text);
		font-size: 0.8rem;
		padding: 3px 8px;
		cursor: pointer;
	}

	.mono {
		font-family: var(--font-mono);
	}
</style>
