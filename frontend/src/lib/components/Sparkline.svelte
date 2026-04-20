<script lang="ts">
	interface Props {
		data: number[]; // Data points
		width?: number; // Default 100
		height?: number; // Default 24
		color?: string; // Default 'var(--accent)'
	}

	let { data, width = 100, height = 24, color = 'var(--accent)' }: Props = $props();

	const points = $derived.by(() => {
		if (data.length === 0) return '';
		const maxVal = Math.max(...data, 1);
		const minVal = Math.min(...data, 0);
		const range = maxVal - minVal || 1;
		return data
			.map((val, i) => {
				const x = (i / Math.max(data.length - 1, 1)) * width;
				const y = height - 2 - ((val - minVal) / range) * (height - 4);
				return `${x.toFixed(1)},${y.toFixed(1)}`;
			})
			.join(' ');
	});
</script>

<svg viewBox="0 0 {width} {height}" class="sparkline">
	<polyline points={points} class="sparkline-line" style:stroke={color} />
</svg>
