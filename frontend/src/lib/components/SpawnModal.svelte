<script lang="ts">
	import { goto } from '$app/navigation';
	import { createSession } from '$lib/api';
	import type { Session, SessionKind, LabelEntry, SandboxSpec, EgressRule } from '$lib/types';

	let {
		open,
		onclose,
		oncreated
	}: {
		open: boolean;
		onclose: () => void;
		oncreated: (session: Session) => void;
	} = $props();

	let slug = $state('');
	let kindType = $state<'shell' | 'claude_code' | 'raw_bytes'>('shell');
	let projectDir = $state('');
	let labelInput = $state('');
	let labels = $state<LabelEntry[]>([]);
	let submitting = $state(false);
	let errorMsg = $state<string | null>(null);

	let sandboxEnabled = $state(false);
	let sandboxImage = $state('ubuntu:24.04');
	let sandboxCpuQuota = $state('');
	let sandboxMemoryMb = $state('');
	let sandboxEgress = $state('');
	let sandboxAllocateTty = $state(false);

	function parseEgressRules(raw: string): EgressRule[] {
		return raw
			.split('\n')
			.map((line) => line.trim())
			.filter((line) => line.length > 0)
			.flatMap((line) => {
				const m = line.match(/^(.+):(\d+)\/(tcp|udp)$/i);
				if (!m) return [];
				return [{ host: m[1], port: parseInt(m[2], 10), proto: m[3].toLowerCase() as 'tcp' | 'udp' }];
			});
	}

	function buildSandboxSpec(): SandboxSpec | undefined {
		if (!sandboxEnabled) return undefined;
		return {
			image: sandboxImage || 'ubuntu:24.04',
			cpu_quota: sandboxCpuQuota ? parseFloat(sandboxCpuQuota) : null,
			memory_limit_mb: sandboxMemoryMb ? parseInt(sandboxMemoryMb, 10) : null,
			egress_allowlist: parseEgressRules(sandboxEgress),
			allocate_tty: sandboxAllocateTty
		};
	}

	function buildKind(): SessionKind {
		if (kindType === 'claude_code') {
			return { type: 'claude_code', project_dir: projectDir || null };
		}
		if (kindType === 'raw_bytes') return { type: 'raw_bytes' };
		return { type: 'shell' };
	}

	function addLabel() {
		const trimmed = labelInput.trim();
		if (!trimmed) return;
		const eqIdx = trimmed.indexOf('=');
		if (eqIdx > 0) {
			labels = [...labels, { key: trimmed.slice(0, eqIdx), value: trimmed.slice(eqIdx + 1) }];
		} else {
			labels = [...labels, { key: trimmed, value: '' }];
		}
		labelInput = '';
	}

	function removeLabel(idx: number) {
		labels = labels.filter((_, i) => i !== idx);
	}

	async function submit() {
		errorMsg = null;
		if (!slug.match(/^[a-z0-9-]{1,50}$/)) {
			errorMsg = 'Slug must be lowercase alphanumeric + hyphens, 1-50 chars.';
			return;
		}
		submitting = true;
		try {
			const session = await createSession({ slug, kind: buildKind(), sandbox: buildSandboxSpec() });
			oncreated(session);
			await goto(`/session/${session.id}`);
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : 'Failed to create session';
		} finally {
			submitting = false;
		}
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window on:keydown={onkeydown} />

{#if open}
	<div
		class="overlay"
		role="presentation"
		onclick={(e) => { if (e.target === e.currentTarget) onclose(); }}
	>
		<div class="modal" role="dialog" aria-modal="true" aria-label="Create session">
			<div class="modal-header">
				<h2>New Session</h2>
				<button class="close-btn" onclick={onclose} aria-label="Close">✕</button>
			</div>

			<form onsubmit={(e) => { e.preventDefault(); submit(); }}>
				<div class="field">
					<label for="slug">Slug</label>
					<input
						id="slug"
						type="text"
						bind:value={slug}
						placeholder="my-session"
						autocomplete="off"
						required
					/>
					<span class="field-hint">Lowercase alphanumeric + hyphens, 1–50 chars</span>
				</div>

				<fieldset class="field field-fieldset">
					<legend class="field-legend">Kind</legend>
					<div class="radio-group">
						<label class="radio-option">
							<input type="radio" bind:group={kindType} value="shell" />
							Shell
						</label>
						<label class="radio-option">
							<input type="radio" bind:group={kindType} value="claude_code" />
							Claude Code
						</label>
						<label class="radio-option">
							<input type="radio" bind:group={kindType} value="raw_bytes" />
							Raw Bytes
						</label>
					</div>
				</fieldset>

				<div class="field">
					<label for="project-dir">
						{kindType === 'claude_code' ? 'Project Directory' : 'Working Directory'}
					</label>
					<input
						id="project-dir"
						type="text"
						bind:value={projectDir}
						placeholder="/home/user/project"
						autocomplete="off"
					/>
				</div>

				<div class="field">
					<label for="label-input">Labels <span class="muted">(saved after creation)</span></label>
					<div class="label-input-row">
						<input
							id="label-input"
							type="text"
							bind:value={labelInput}
							placeholder="key=value or tag"
							onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addLabel(); } }}
						/>
						<button type="button" class="btn-add" onclick={addLabel}>Add</button>
					</div>
					{#if labels.length > 0}
						<div class="label-chips">
							{#each labels as label, i}
								<span class="label-chip">
									{label.value ? `${label.key}=${label.value}` : label.key}
									<button type="button" class="chip-remove" onclick={() => removeLabel(i)}>×</button>
								</span>
							{/each}
						</div>
					{/if}
				</div>

				<fieldset class="field field-fieldset">
					<legend class="field-legend">Sandbox</legend>
					<label class="radio-option">
						<input type="checkbox" bind:checked={sandboxEnabled} />
						Enable sandbox (podman + overlayfs)
					</label>
					{#if sandboxEnabled}
						<div class="sandbox-fields">
							<div class="field">
								<label for="sandbox-image">Image</label>
								<input
									id="sandbox-image"
									type="text"
									bind:value={sandboxImage}
									placeholder="ubuntu:24.04"
									autocomplete="off"
								/>
							</div>
							<div class="field">
								<label for="sandbox-cpu">CPU quota (fractional CPUs)</label>
								<input
									id="sandbox-cpu"
									type="number"
									step="0.5"
									min="0.1"
									bind:value={sandboxCpuQuota}
									placeholder="2.0"
								/>
							</div>
							<div class="field">
								<label for="sandbox-mem">Memory limit (MB)</label>
								<input
									id="sandbox-mem"
									type="number"
									min="64"
									bind:value={sandboxMemoryMb}
									placeholder="4096"
								/>
							</div>
							<div class="field">
								<label for="sandbox-egress">Egress allowlist <span class="muted">(one per line: host:port/proto)</span></label>
								<textarea
									id="sandbox-egress"
									bind:value={sandboxEgress}
									rows="3"
									placeholder="api.anthropic.com:443/tcp"
								></textarea>
								<span class="field-hint">Empty = no network (--network=none)</span>
							</div>
							<label class="radio-option">
								<input type="checkbox" bind:checked={sandboxAllocateTty} />
								Allocate TTY (for interactive shells)
							</label>
						</div>
					{/if}
				</fieldset>

				{#if errorMsg}
					<p class="error">{errorMsg}</p>
				{/if}

				<div class="modal-actions">
					<button type="button" class="btn-cancel" onclick={onclose}>Cancel</button>
					<button type="submit" class="btn-submit" disabled={submitting}>
						{submitting ? 'Creating…' : 'Create Session'}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 24px;
		width: 100%;
		max-width: 480px;
		margin: 16px;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 20px;
	}

	h2 {
		margin: 0;
		font-size: 1.1rem;
		color: var(--text);
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 1rem;
		padding: 4px 8px;
	}

	.close-btn:hover {
		color: var(--text);
	}

	.field {
		margin-bottom: 16px;
	}

	.field-fieldset {
		border: none;
		padding: 0;
		margin: 0 0 16px 0;
	}

	.field-legend {
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-bottom: 6px;
		padding: 0;
	}

	label {
		display: block;
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-bottom: 6px;
	}

	.muted {
		color: var(--text-muted);
		font-style: italic;
	}

	input[type='text'] {
		width: 100%;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		padding: 8px 10px;
		outline: none;
	}

	input[type='text']:focus {
		border-color: var(--accent);
	}

	.field-hint {
		display: block;
		font-size: 0.72rem;
		color: var(--text-muted);
		margin-top: 4px;
	}

	.radio-group {
		display: flex;
		gap: 16px;
	}

	.radio-option {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.85rem;
		color: var(--text);
		cursor: pointer;
		margin-bottom: 0;
	}

	.label-input-row {
		display: flex;
		gap: 8px;
	}

	.label-input-row input {
		flex: 1;
	}

	.btn-add {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		padding: 6px 12px;
		font-size: 0.8rem;
	}

	.btn-add:hover {
		border-color: var(--accent);
		color: var(--text);
	}

	.label-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		margin-top: 8px;
	}

	.label-chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 2px 8px;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 10px;
		color: var(--text);
	}

	.chip-remove {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		padding: 0;
		font-size: 0.85rem;
		line-height: 1;
	}

	.chip-remove:hover {
		color: var(--text);
	}

	.error {
		color: #f44336;
		font-size: 0.85rem;
		margin: 0 0 12px;
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 8px;
	}

	.btn-cancel {
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		padding: 8px 16px;
		font-size: 0.85rem;
	}

	.btn-cancel:hover {
		border-color: var(--text);
		color: var(--text);
	}

	.btn-submit {
		background: var(--accent);
		border: none;
		border-radius: var(--radius);
		color: #000;
		cursor: pointer;
		font-size: 0.85rem;
		font-weight: 600;
		padding: 8px 18px;
	}

	.btn-submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-submit:not(:disabled):hover {
		opacity: 0.85;
	}

	.sandbox-fields {
		margin-top: 12px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	textarea {
		width: 100%;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		padding: 8px 10px;
		outline: none;
		resize: vertical;
	}

	textarea:focus {
		border-color: var(--accent);
	}

	input[type='number'] {
		width: 100%;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		padding: 8px 10px;
		outline: none;
	}

	input[type='number']:focus {
		border-color: var(--accent);
	}
</style>
