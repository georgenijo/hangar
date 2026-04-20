<script lang="ts">
	interface BarData {
		label: string;
		value: number;
	}

	interface Props {
		data: BarData[];
		width?: number;
		height?: number;
		horizontal?: boolean;
	}

	let { data, width = 600, height = 200, horizontal = false }: Props = $props();

	// Compute max value for normalization
	const maxValue = $derived(Math.max(...data.map((d) => d.value), 1));

	// Generate bars
	const bars = $derived.by(() => {
		if (horizontal) {
			// Horizontal bars
			const barHeight = height / data.length;
			return data.map((d, i) => {
				const barWidth = (d.value / maxValue) * (width - 100); // Leave space for labels
				return {
					x: 80,
					y: i * barHeight + barHeight * 0.2,
					width: barWidth,
					height: barHeight * 0.6,
					label: d.label,
					labelX: 5,
					labelY: i * barHeight + barHeight * 0.5,
					valueX: 85 + barWidth,
					valueY: i * barHeight + barHeight * 0.5
				};
			});
		} else {
			// Vertical bars
			const barWidth = width / data.length;
			return data.map((d, i) => {
				const barHeight = (d.value / maxValue) * (height - 40); // Leave space for labels
				return {
					x: i * barWidth + barWidth * 0.2,
					y: height - barHeight - 20,
					width: barWidth * 0.6,
					height: barHeight,
					label: d.label,
					labelX: i * barWidth + barWidth * 0.5,
					labelY: height - 5
				};
			});
		}
	});
</script>

<svg viewBox="0 0 {width} {height}" class="bar-chart" class:horizontal>
	{#each bars as bar, i}
		<!-- Bar -->
		<rect
			x={bar.x}
			y={bar.y}
			width={bar.width}
			height={bar.height}
			class="bar"
			class:bar-accent={i >= data.length - 2}
			rx="2"
		/>

		<!-- Labels -->
		{#if horizontal}
			<text x={bar.labelX} y={bar.labelY} class="bar-label" text-anchor="start">{bar.label}</text>
		{:else}
			<text x={bar.labelX} y={bar.labelY} class="bar-label" text-anchor="middle">{bar.label}</text
			>
		{/if}
	{/each}
</svg>

<style>
	.bar-chart {
		width: 100%;
		display: block;
	}

	.bar-label {
		font-family: var(--mono);
		font-size: 9px;
		fill: var(--text-3);
	}
</style>
