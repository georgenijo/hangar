<script lang="ts">
	let {
		label,
		value,
		sublabel,
		intent = 'default'
	}: {
		label: string;
		value: string | number;
		sublabel?: string;
		intent?: 'default' | 'accent' | 'warn';
	} = $props();
</script>

<div class="kpi-card kpi-{intent}">
	<div class="kpi-label">{label}</div>
	<div class="kpi-value">{value}</div>
	{#if sublabel}
		<div class="kpi-sub">{sublabel}</div>
	{/if}
</div>

<style>
	.kpi-card {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 14px 16px;
	}

	.kpi-label {
		font-size: 0.78rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 6px;
	}

	.kpi-value {
		font-family: var(--font-mono);
		font-size: 1.6rem;
		font-weight: 600;
		color: var(--text);
		line-height: 1.1;
	}

	.kpi-accent .kpi-value {
		color: var(--accent);
	}

	.kpi-warn .kpi-value {
		color: #ff9800;
	}

	.kpi-sub {
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-top: 4px;
	}
</style>
