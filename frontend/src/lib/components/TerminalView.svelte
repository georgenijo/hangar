<script lang="ts">
	import { onMount } from 'svelte';
	import type { Session } from '$lib/types';
	import { getSessionOutput, resizeSession } from '$lib/api';

	let { session }: { session: Session } = $props();

	let container: HTMLElement;
	let status = $state<'connecting' | 'connected' | 'disconnected' | 'failed'>('connecting');

	onMount(() => {
		let term: import('@xterm/xterm').Terminal | null = null;
		let fitAddon: import('@xterm/addon-fit').FitAddon | null = null;
		let ws: WebSocket | null = null;
		let lastKnownOffset = 0;
		let reconnectAttempts = 0;
		let destroyed = false;
		let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

		async function connect() {
			if (destroyed) return;
			if (session.state === 'exited') {
				status = 'disconnected';
				return;
			}

			status = 'connecting';
			const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
			const wsUrl = `${proto}//${location.host}/ws/v1/sessions/${session.id}/pty`;

			ws = new WebSocket(wsUrl);
			ws.binaryType = 'arraybuffer';

			ws.onopen = () => {
				if (!term || !fitAddon) return;
				status = 'connected';
				reconnectAttempts = 0;
				fitAddon.fit();
				resizeSession(session.id, term.cols, term.rows).catch(() => {});
				ws!.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }));
			};

			ws.onmessage = (e) => {
				if (e.data instanceof ArrayBuffer) {
					const bytes = new Uint8Array(e.data);
					term?.write(bytes);
				}
			};

			ws.onerror = () => {
				ws?.close();
			};

			ws.onclose = () => {
				if (destroyed) return;
				scheduleReconnect();
			};
		}

		async function scheduleReconnect() {
			if (destroyed) return;
			if (reconnectAttempts >= 5) {
				status = 'failed';
				return;
			}

			status = 'disconnected';
			const delay = Math.pow(2, reconnectAttempts) * 1000;
			reconnectAttempts++;

			reconnectTimer = setTimeout(async () => {
				if (destroyed) return;
				try {
					const { data, head } = await getSessionOutput(session.id, { offset: lastKnownOffset });
					if (data.byteLength > 0) {
						term?.write(new Uint8Array(data));
					}
					lastKnownOffset = head;
				} catch {
					// ignore — connect anyway
				}
				connect();
			}, delay);
		}

		async function init() {
			const { Terminal } = await import('@xterm/xterm');
			const { FitAddon } = await import('@xterm/addon-fit');

			if (destroyed) return;

			// Load xterm CSS
			const link = document.createElement('link');
			link.rel = 'stylesheet';
			link.href = new URL('@xterm/xterm/css/xterm.css', import.meta.url).href;
			document.head.appendChild(link);

			term = new Terminal({
				theme: {
					background: '#0e0e10',
					foreground: '#eeeeee',
					cursor: '#99ccff',
					selectionBackground: '#2a2a30'
				},
				fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
				fontSize: 13,
				cursorBlink: true
			});

			fitAddon = new FitAddon();
			term.loadAddon(fitAddon);
			term.open(container);
			fitAddon.fit();

			// Fetch initial output
			try {
				const { data, head } = await getSessionOutput(session.id);
				if (data.byteLength > 0) {
					term.write(new Uint8Array(data));
				}
				lastKnownOffset = head;
			} catch {
				// proceed without initial output
			}

			term.onData((data: string) => {
				if (ws && ws.readyState === WebSocket.OPEN) {
					const enc = new TextEncoder();
					ws.send(enc.encode(data));
				}
			});

			const resizeObserver = new ResizeObserver(() => {
				if (!fitAddon || !term || !ws) return;
				fitAddon.fit();
				if (ws.readyState === WebSocket.OPEN) {
					ws.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }));
					resizeSession(session.id, term.cols, term.rows).catch(() => {});
				}
			});
			resizeObserver.observe(container);

			connect();

			return () => resizeObserver.disconnect();
		}

		let cleanupObserver: (() => void) | undefined;
		init().then((cleanup) => {
			cleanupObserver = cleanup;
		});

		return () => {
			destroyed = true;
			if (reconnectTimer) clearTimeout(reconnectTimer);
			ws?.close();
			term?.dispose();
			cleanupObserver?.();
		};
	});
</script>

<div class="terminal-wrapper">
	{#if status === 'failed'}
		<div class="overlay">
			<p>Disconnected</p>
			<small>Max reconnect attempts reached</small>
		</div>
	{/if}
	{#if status === 'disconnected'}
		<div class="status-bar">Reconnecting…</div>
	{/if}
	<div class="terminal-container" bind:this={container}></div>
</div>

<style>
	.terminal-wrapper {
		flex: 1;
		display: flex;
		flex-direction: column;
		background: #0e0e10;
		position: relative;
		overflow: hidden;
	}

	.terminal-container {
		flex: 1;
		overflow: hidden;
		padding: 4px;
	}

	.overlay {
		position: absolute;
		inset: 0;
		background: rgba(14, 14, 16, 0.85);
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 8px;
		color: var(--text-muted);
		z-index: 10;
	}

	.overlay p {
		margin: 0;
		font-size: 1rem;
		color: var(--text);
	}

	.overlay small {
		font-size: 0.8rem;
	}

	.status-bar {
		background: rgba(255, 152, 0, 0.15);
		border-bottom: 1px solid rgba(255, 152, 0, 0.4);
		color: #ff9800;
		font-size: 0.75rem;
		padding: 4px 12px;
		text-align: center;
		flex-shrink: 0;
	}
</style>
