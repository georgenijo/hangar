<script lang="ts">
	interface Props {
		value: number; // 0-100 percentage
		label: string; // Display label
		warn_threshold?: number; // Default 75
		size?: number; // SVG size in px, default 60
	}

	let { value, label, warn_threshold = 75, size = 60 }: Props = $props();

	const radius = 26;
	const circumference = 2 * Math.PI * radius; // ~163.36
	const offset = $derived(circumference * (1 - value / 100));
	const isWarn = $derived(value >= warn_threshold);
</script>

<div class="ring-metric">
	<svg viewBox="0 0 60 60" class="ring" style:width="{size}px" style:height="{size}px">
		<circle cx="30" cy="30" r={radius} class="ring-bg" />
		<circle
			cx="30"
			cy="30"
			r={radius}
			class="ring-fg"
			class:ring-warn={isWarn}
			stroke-dasharray={circumference}
			stroke-dashoffset={offset}
			style="transform: rotate(-90deg); transform-origin: center;"
		/>
	</svg>
	<div class="ring-val">{Math.round(value)}%</div>
	<div class="ring-label">{label}</div>
</div>
