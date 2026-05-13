<script lang="ts">
	let {
		pct,
		label,
		size = 88,
		strokeWidth = 8,
		sublabel = ''
	}: {
		pct: number;
		label: string;
		size?: number;
		strokeWidth?: number;
		sublabel?: string;
	} = $props();

	function thresholdColor(p: number): string {
		if (p >= 90) return '#f44336';
		if (p >= 75) return '#ff9800';
		if (p >= 50) return '#f5c518';
		return '#4caf50';
	}

	let clamped = $derived(Number.isFinite(pct) ? Math.max(0, Math.min(100, pct)) : 0);
	let radius = $derived((size - strokeWidth) / 2);
	let circumference = $derived(2 * Math.PI * radius);
	let offset = $derived(circumference * (1 - clamped / 100));
	let color = $derived(thresholdColor(clamped));
	let cx = $derived(size / 2);
	let cy = $derived(size / 2);
	let displayText = $derived(Number.isFinite(pct) ? `${Math.round(clamped)}%` : '—%');
</script>

<div class="gauge">
	<svg width={size} height={size} viewBox="0 0 {size} {size}">
		<circle
			cx={cx}
			cy={cy}
			r={radius}
			fill="none"
			stroke="var(--border)"
			stroke-width={strokeWidth}
		/>
		<circle
			cx={cx}
			cy={cy}
			r={radius}
			fill="none"
			stroke={color}
			stroke-width={strokeWidth}
			stroke-linecap="round"
			stroke-dasharray={circumference}
			stroke-dashoffset={offset}
			transform="rotate(-90 {cx} {cy})"
		/>
		<text
			x={cx}
			y={cy}
			dominant-baseline="middle"
			text-anchor="middle"
			font-size="13"
			font-family="var(--font-mono)"
			fill="var(--text)"
		>{displayText}</text>
	</svg>
	<div class="gauge-label">{label}</div>
	{#if sublabel}
		<div class="gauge-sub">{sublabel}</div>
	{/if}
</div>

<style>
	.gauge {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
	}

	.gauge-label {
		font-size: 0.78rem;
		color: var(--text-muted);
		font-family: var(--font-sans);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.gauge-sub {
		font-size: 0.7rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
</style>
