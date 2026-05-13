<script lang="ts">
	import type { DailyCost } from '$lib/types';

	let {
		days,
		width = 480,
		height = 160
	}: {
		days: DailyCost[];
		width?: number;
		height?: number;
	} = $props();

	const MARGIN = { top: 12, right: 8, bottom: 22, left: 36 };

	function fmtDay(iso: string): string {
		return iso.slice(5);
	}

	let visible = $derived(days.slice(0, 7).reverse());
	let pw = $derived(width - MARGIN.left - MARGIN.right);
	let ph = $derived(height - MARGIN.top - MARGIN.bottom);
	let yMax = $derived(Math.max(...visible.map((d) => (Number.isFinite(d.usd) ? d.usd : 0)), 0.01));
	let barCount = $derived(visible.length);
	let slotWidth = $derived(barCount > 0 ? pw / barCount : pw);
	let barWidth = $derived(slotWidth * 0.65);

	function barX(i: number): number {
		return MARGIN.left + i * slotWidth + (slotWidth - barWidth) / 2;
	}

	function barH(usd: number): number {
		return (usd / yMax) * ph;
	}

	function barY(usd: number): number {
		return MARGIN.top + ph - barH(usd);
	}

	function labelX(i: number): number {
		return MARGIN.left + i * slotWidth + slotWidth / 2;
	}

	let gridLines = $derived([0, 0.5, 1].map((f) => ({
		y: MARGIN.top + ph * (1 - f),
		label: f === 0 ? '$0' : `$${(yMax * f).toFixed(2)}`
	})));
</script>

{#if visible.length === 0}
	<div class="empty">No cost data yet</div>
{:else}
	<svg {width} {height} viewBox="0 0 {width} {height}" style="width: 100%; max-width: {width}px;">
		<!-- grid lines -->
		{#each gridLines as gl}
			<line
				x1={MARGIN.left}
				y1={gl.y}
				x2={width - MARGIN.right}
				y2={gl.y}
				stroke="var(--border)"
				stroke-width="1"
			/>
			<text
				x={MARGIN.left - 4}
				y={gl.y}
				text-anchor="end"
				dominant-baseline="middle"
				font-size="9"
				font-family="var(--font-mono)"
				fill="var(--text-muted)"
			>{gl.label}</text>
		{/each}

		<!-- bars -->
		{#each visible as d, i}
			<g>
				<title>{d.date}: ${d.usd.toFixed(4)} · {d.tokens.toLocaleString()} tok</title>
				<rect
					x={barX(i)}
					y={barY(d.usd)}
					width={barWidth}
					height={Math.max(barH(d.usd), 1)}
					fill="var(--accent)"
					rx="2"
					opacity="0.85"
				/>
			</g>
			<!-- x-axis label -->
			{#if barCount <= 5 || i % 2 === 0}
				<text
					x={labelX(i)}
					y={height - MARGIN.bottom + 14}
					text-anchor="middle"
					font-size="9"
					font-family="var(--font-mono)"
					fill="var(--text-muted)"
				>{fmtDay(d.date)}</text>
			{/if}
		{/each}
	</svg>
{/if}

<style>
	.empty {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 80px;
		color: var(--text-muted);
		font-size: 0.85rem;
	}
</style>
