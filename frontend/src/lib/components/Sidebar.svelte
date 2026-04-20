<script lang="ts">
	import type { ViewId, NavItem } from '$lib/types';
	import { currentView } from '$lib/router.svelte';

	const NAV_ITEMS: NavItem[] = [
		{ id: 'command', label: 'Command', icon: '◎' },
		{ id: 'sessions', label: 'Sessions', icon: '⬡' },
		{ id: 'pipeline', label: 'Pipelines', icon: '◈' },
		{ id: 'conductor', label: 'Conductor', icon: '◇' },
		{ id: 'fleet', label: 'Fleet', icon: '▣' },
		{ id: 'search', label: 'Search', icon: '⌕' },
		{ id: 'costs', label: 'Costs', icon: '$' },
		{ id: 'voice', label: 'Voice', icon: '🎤' },
		{ id: 'settings', label: 'Settings', icon: '⚙' }
	];
</script>

<aside class="sidebar">
	<div class="brand">
		<span class="mark">H</span>
		<span class="wordmark">Hangar</span>
	</div>
	<nav>
		<ul class="nav-list">
			{#each NAV_ITEMS as item}
				<li>
					<button
						class="nav-link"
						class:active={$currentView === item.id}
						onclick={() => currentView.navigate(item.id)}
						type="button"
					>
						<span class="icon">{item.icon}</span>
						<span>{item.label}</span>
					</button>
				</li>
			{/each}
		</ul>
	</nav>
	<div class="nav-footer">
		<div class="nav-user">
			<div class="avatar">G</div>
			<div class="nav-user-meta">
				<div class="nav-user-name">george</div>
				<div class="nav-user-sub">local</div>
			</div>
		</div>
	</div>
</aside>
