<script lang="ts">
	import type { PipelinePhase } from '$lib/types';

	interface Props {
		phases: PipelinePhase[];
	}

	let { phases }: Props = $props();
</script>

<div class="phase-track">
	{#each phases as phase, i}
		<div class="phase-node {phase.state}">
			<span class="phase-dot"></span>
			<span class="phase-name">{phase.name}</span>
			{#if phase.info}
				<span class="phase-info">{phase.info}</span>
			{/if}
		</div>
		{#if i < phases.length - 1}
			<div class="phase-line {phase.state}"></div>
		{/if}
	{/each}
</div>
