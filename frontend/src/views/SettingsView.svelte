<script lang="ts">
	// SettingsView: Static mock UI for settings configuration
	// Per PRD: toggles and inputs render but do not persist (static UI only)

	let activeSection = $state('general');

	const navItems = [
		{ id: 'general', label: 'General' },
		{ id: 'models', label: 'Models' },
		{ id: 'fleet', label: 'Fleet' },
		{ id: 'dispatch', label: 'Dispatch' },
		{ id: 'sandbox', label: 'Sandbox' },
		{ id: 'notifications', label: 'Notifications' },
		{ id: 'mcp', label: 'MCP' },
		{ id: 'api-keys', label: 'API Keys' },
		{ id: 'danger', label: 'Danger Zone' }
	];

	// Mock state for UI (doesn't persist per PRD)
	let selectedModel = $state('opus');
	let selectedHost = $state('optiplex');
	let softBudget = $state(50);
	let hardBudget = $state(100);
	let notifyOnComplete = $state(true);
	let notifyOnError = $state(true);
	let notifyOnReview = $state(false);
</script>

<div id="view-settings" class="view">
	<div class="settings-layout">
		<!-- Left Navigation -->
		<nav class="settings-nav">
			{#each navItems as item}
				<button
					class="settings-link"
					class:settings-active={activeSection === item.id}
					onclick={() => activeSection = item.id}
				>
					{item.label}
				</button>
			{/each}
		</nav>

		<!-- Right Body -->
		<div class="settings-body">
			{#if activeSection === 'general'}
				<div class="setting-section">
					<div class="setting-title">Theme</div>
					<div class="setting-desc">Dark theme is enabled by default. Light mode toggle coming soon.</div>
					<div class="select-mock">
						<span>Dark (default)</span>
						<span class="chev">▼</span>
					</div>
				</div>
			{/if}

			{#if activeSection === 'models'}
				<div class="setting-section">
					<div class="setting-title">Default Model</div>
					<div class="setting-desc">Select the default model for new sessions.</div>
					<div class="radio-grid">
						<label class="radio-card" class:radio-on={selectedModel === 'opus'}>
							<input type="radio" name="model" value="opus" bind:group={selectedModel} />
							<div class="radio-h">Opus 4.7</div>
							<div class="radio-s">Most capable</div>
						</label>
						<label class="radio-card" class:radio-on={selectedModel === 'sonnet'}>
							<input type="radio" name="model" value="sonnet" bind:group={selectedModel} />
							<div class="radio-h">Sonnet 4.5</div>
							<div class="radio-s">Balanced</div>
						</label>
						<label class="radio-card" class:radio-on={selectedModel === 'haiku'}>
							<input type="radio" name="model" value="haiku" bind:group={selectedModel} />
							<div class="radio-h">Haiku 4.0</div>
							<div class="radio-s">Fast</div>
						</label>
					</div>
				</div>
			{/if}

			{#if activeSection === 'fleet'}
				<div class="setting-section">
					<div class="setting-title">Default Host</div>
					<div class="setting-desc">Select the default host for new sessions.</div>
					<div class="select-mock">
						<span>{selectedHost}</span>
						<span class="chev">▼</span>
					</div>
				</div>
			{/if}

			{#if activeSection === 'dispatch'}
				<div class="setting-section">
					<div class="setting-title">Session Budgets</div>
					<div class="setting-desc">Set token and cost limits for sessions.</div>
					<div class="range-row">
						<div class="range-label">Soft limit:</div>
						<input type="range" min="10" max="100" bind:value={softBudget} />
						<div class="range-val">{softBudget}k</div>
					</div>
					<div class="range-row">
						<div class="range-label">Hard limit:</div>
						<input type="range" min="20" max="200" bind:value={hardBudget} />
						<div class="range-val">{hardBudget}k</div>
					</div>
				</div>
			{/if}

			{#if activeSection === 'sandbox'}
				<div class="setting-section">
					<div class="setting-title">Sandbox Configuration</div>
					<div class="setting-desc">Linux-only feature. macOS sessions run without sandbox.</div>
					<div class="hint-text">Sandbox manager is available on Linux hosts with podman.</div>
				</div>
			{/if}

			{#if activeSection === 'notifications'}
				<div class="setting-section">
					<div class="setting-title">Notification Preferences</div>
					<div class="setting-desc">Choose when to receive notifications.</div>
					<div class="toggle-list">
						<div class="toggle-row">
							<span>Session complete</span>
							<button
								class="toggle"
								class:toggle-on={notifyOnComplete}
								onclick={() => notifyOnComplete = !notifyOnComplete}
								aria-label="Toggle session complete notifications"
							>
								<div class="toggle-dot"></div>
							</button>
						</div>
						<div class="toggle-row">
							<span>Session error</span>
							<button
								class="toggle"
								class:toggle-on={notifyOnError}
								onclick={() => notifyOnError = !notifyOnError}
								aria-label="Toggle session error notifications"
							>
								<div class="toggle-dot"></div>
							</button>
						</div>
						<div class="toggle-row">
							<span>Review required</span>
							<button
								class="toggle"
								class:toggle-on={notifyOnReview}
								onclick={() => notifyOnReview = !notifyOnReview}
								aria-label="Toggle review required notifications"
							>
								<div class="toggle-dot"></div>
							</button>
						</div>
					</div>
				</div>
			{/if}

			{#if activeSection === 'mcp'}
				<div class="setting-section">
					<div class="setting-title">MCP Servers</div>
					<div class="setting-desc">Manage MCP server configurations (Phase 8 feature).</div>
					<div class="hint-text">MCP server support coming soon.</div>
				</div>
			{/if}

			{#if activeSection === 'api-keys'}
				<div class="setting-section">
					<div class="setting-title">API Keys</div>
					<div class="setting-desc">Manage API keys for external integrations.</div>
					<div class="hint-text">No external API keys configured.</div>
				</div>
			{/if}

			{#if activeSection === 'danger'}
				<div class="setting-section">
					<div class="setting-title">Danger Zone</div>
					<div class="setting-desc">Destructive actions that cannot be undone.</div>
					<button class="btn btn-danger-ghost btn-sm">Clear all session data</button>
				</div>
			{/if}
		</div>
	</div>
</div>
