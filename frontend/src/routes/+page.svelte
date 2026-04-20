<script lang="ts">
	import { currentView } from '$lib/router.svelte';
	import CommandView from '../views/CommandView.svelte';
	import SessionsView from '../views/SessionsView.svelte';
	import WorkspaceView from '../views/WorkspaceView.svelte';
	import PipelineView from '../views/PipelineView.svelte';
	import ConductorView from '../views/ConductorView.svelte';
	import FleetView from '../views/FleetView.svelte';
	import SearchView from '../views/SearchView.svelte';
	import CostsView from '../views/CostsView.svelte';
	import VoiceView from '../views/VoiceView.svelte';
	import SettingsView from '../views/SettingsView.svelte';

	const view = $derived($currentView);
</script>

<div class="view-host">
	{#if view === 'command'}
		<CommandView />
	{:else if view === 'sessions'}
		<SessionsView />
	{:else if view === 'session-detail'}
		<WorkspaceView />
	{:else if view === 'pipeline'}
		<PipelineView />
	{:else if view === 'conductor'}
		<ConductorView />
	{:else if view === 'fleet'}
		<FleetView />
	{:else if view === 'search'}
		<SearchView />
	{:else if view === 'costs'}
		<CostsView />
	{:else if view === 'voice'}
		<VoiceView />
	{:else if view === 'settings'}
		<SettingsView />
	{:else}
		<CommandView />
	{/if}
</div>

<style>
	.view-host {
		min-height: 100%;
	}
</style>
