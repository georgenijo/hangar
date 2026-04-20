<script lang="ts">
	import { currentView } from '$lib/router.svelte';

	interface Props {
		title?: string;
		subtitle?: string;
	}

	let { title, subtitle }: Props = $props();

	const VIEW_TITLES: Record<string, string> = {
		command: 'Command Center',
		sessions: 'Sessions',
		'session-detail': 'Workspace',
		pipeline: 'Pipelines',
		conductor: 'Conductor',
		fleet: 'Fleet',
		search: 'Search',
		costs: 'Costs',
		voice: 'Voice',
		settings: 'Settings'
	};

	const displayTitle = $derived(title ?? VIEW_TITLES[$currentView] ?? 'Hangar');
</script>

<header class="topbar">
	<div class="topbar-left">
		<div class="crumb">
			<span class="crumb-current">{displayTitle}</span>
		</div>
		{#if subtitle}
			<span class="subtitle">{subtitle}</span>
		{/if}
	</div>
	<div class="topbar-search">
		<span class="search-icon">⌕</span>
		<input type="text" placeholder="Search..." />
	</div>
	<div class="topbar-actions">
		<button class="btn btn-ghost btn-sm">
			<span>⚙</span>
		</button>
	</div>
</header>
