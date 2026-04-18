<script lang="ts">
	import { getWorktreeTree, getWorktreeFile, getWorktreeDiff, ApiError } from '$lib/api';
	import type {
		WorktreeTreeResponse,
		WorktreeEntry,
		WorktreeFileResponse,
		WorktreeDiffResponse,
		WorktreeFileStatus
	} from '$lib/types';

	let { sessionId }: { sessionId: string } = $props();

	let tree = $state<WorktreeTreeResponse | null>(null);
	let treeError = $state<string | null>(null);
	let loadingTree = $state(false);

	let selected = $state<string | null>(null);
	let mode = $state<'file' | 'diff'>('diff');
	let file = $state<WorktreeFileResponse | null>(null);
	let diff = $state<WorktreeDiffResponse | null>(null);
	let fileError = $state<string | null>(null);
	let loadingFile = $state(false);

	// Build a folder tree from flat paths.
	interface TreeNode {
		name: string;
		path: string;
		isDir: boolean;
		status?: WorktreeFileStatus;
		children: TreeNode[];
	}

	let collapsed = $state<Record<string, boolean>>({});

	function buildTree(entries: WorktreeEntry[]): TreeNode {
		const root: TreeNode = { name: '', path: '', isDir: true, children: [] };
		for (const e of entries) {
			const parts = e.path.split('/');
			let cur = root;
			for (let i = 0; i < parts.length; i++) {
				const part = parts[i];
				const isLast = i === parts.length - 1;
				let next = cur.children.find((c) => c.name === part);
				if (!next) {
					next = {
						name: part,
						path: parts.slice(0, i + 1).join('/'),
						isDir: !isLast,
						status: isLast ? e.status : undefined,
						children: []
					};
					cur.children.push(next);
				}
				cur = next;
			}
		}
		sortNode(root);
		return root;
	}

	function sortNode(n: TreeNode) {
		n.children.sort((a, b) => {
			if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
			return a.name.localeCompare(b.name);
		});
		for (const c of n.children) sortNode(c);
	}

	// Folders that contain any dirty descendant should show a dot too.
	function dirtyFolder(n: TreeNode): boolean {
		if (!n.isDir) return false;
		return n.children.some(
			(c) =>
				(!c.isDir && c.status && c.status !== 'clean') ||
				(c.isDir && dirtyFolder(c))
		);
	}

	let rootNode = $derived(tree ? buildTree(tree.entries) : null);
	let dirtyCount = $derived(
		tree ? tree.entries.filter((e) => e.status !== 'clean').length : 0
	);

	async function loadTree() {
		loadingTree = true;
		treeError = null;
		try {
			tree = await getWorktreeTree(sessionId);
		} catch (e) {
			tree = null;
			treeError =
				e instanceof ApiError
					? e.status === 404
						? 'This session is not inside a git worktree.'
						: `${e.status}: ${e.body || e.message}`
					: e instanceof Error
						? e.message
						: 'Failed to load tree';
		} finally {
			loadingTree = false;
		}
	}

	async function loadPath(path: string) {
		selected = path;
		fileError = null;
		loadingFile = true;
		file = null;
		diff = null;

		// Pick default mode: diff if the file is dirty, else file.
		const entry = tree?.entries.find((e) => e.path === path);
		mode =
			entry && entry.status !== 'clean' && entry.status !== 'deleted' ? 'diff' : 'file';
		if (entry?.status === 'deleted') mode = 'diff';

		try {
			if (mode === 'file') {
				file = await getWorktreeFile(sessionId, path);
			} else {
				diff = await getWorktreeDiff(sessionId, path);
			}
		} catch (e) {
			fileError =
				e instanceof ApiError
					? `${e.status}: ${e.body || e.message}`
					: e instanceof Error
						? e.message
						: 'Failed to load file';
		} finally {
			loadingFile = false;
		}
	}

	async function switchMode(next: 'file' | 'diff') {
		if (next === mode || !selected) {
			mode = next;
			return;
		}
		mode = next;
		fileError = null;
		loadingFile = true;
		try {
			if (next === 'file') {
				if (!file) file = await getWorktreeFile(sessionId, selected);
			} else {
				if (!diff) diff = await getWorktreeDiff(sessionId, selected);
			}
		} catch (e) {
			fileError =
				e instanceof ApiError
					? `${e.status}: ${e.body || e.message}`
					: e instanceof Error
						? e.message
						: 'Failed to load file';
		} finally {
			loadingFile = false;
		}
	}

	function toggleCollapse(path: string) {
		collapsed[path] = !collapsed[path];
	}

	function statusDotClass(s: WorktreeFileStatus | undefined): string {
		if (!s) return '';
		if (s === 'clean') return '';
		return `dot-${s}`;
	}

	function statusTitle(s: WorktreeFileStatus | undefined): string {
		switch (s) {
			case 'added':
			case 'untracked':
				return 'added';
			case 'modified':
			case 'renamed':
				return 'modified';
			case 'deleted':
				return 'deleted';
			default:
				return '';
		}
	}

	function diffLineClass(line: string): string {
		if (line.startsWith('+++') || line.startsWith('---')) return 'diff-meta';
		if (line.startsWith('@@')) return 'diff-hunk';
		if (line.startsWith('+')) return 'diff-add';
		if (line.startsWith('-')) return 'diff-del';
		if (line.startsWith('diff ') || line.startsWith('index ')) return 'diff-meta';
		return '';
	}

	$effect(() => {
		// Reload when sessionId changes
		sessionId;
		loadTree();
	});
</script>

<div class="code-viewer">
	<div class="cv-header">
		<span class="cv-title">Code</span>
		{#if tree?.branch}
			<span class="cv-branch" title="Branch">⎇ {tree.branch}</span>
		{/if}
		{#if tree}
			<span class="cv-count">{dirtyCount} changed / {tree.entries.length} files</span>
		{/if}
		<button class="cv-refresh" onclick={loadTree} disabled={loadingTree} title="Refresh">
			{loadingTree ? '…' : '↻'}
		</button>
	</div>

	<div class="cv-body">
		<aside class="cv-tree">
			{#if loadingTree && !tree}
				<div class="cv-loading">Loading…</div>
			{:else if treeError}
				<div class="cv-error">{treeError}</div>
			{:else if rootNode}
				{@render renderNode(rootNode, 0)}
			{/if}
		</aside>

		<section class="cv-pane">
			{#if !selected}
				<div class="cv-placeholder">Select a file from the tree.</div>
			{:else}
				<div class="cv-pane-header">
					<span class="cv-pane-path mono">{selected}</span>
					<div class="cv-mode-toggle">
						<button
							class:active={mode === 'file'}
							onclick={() => switchMode('file')}
							disabled={loadingFile}
						>
							File
						</button>
						<button
							class:active={mode === 'diff'}
							onclick={() => switchMode('diff')}
							disabled={loadingFile}
						>
							Diff
						</button>
					</div>
				</div>
				{#if fileError}
					<div class="cv-error">{fileError}</div>
				{:else if loadingFile}
					<div class="cv-loading">Loading…</div>
				{:else if mode === 'file' && file}
					{#if file.binary}
						<div class="cv-placeholder">Binary file ({file.size} bytes) — not rendered.</div>
					{:else}
						<pre class="cv-file mono">{file.content}</pre>
						{#if file.truncated}
							<div class="cv-notice">File truncated at 500 KB.</div>
						{/if}
					{/if}
				{:else if mode === 'diff' && diff}
					{#if diff.diff === ''}
						<div class="cv-placeholder">No diff vs HEAD.</div>
					{:else}
						<pre class="cv-diff mono">{#each diff.diff.split('\n') as line, i (i)}<span
									class={diffLineClass(line)}>{line}
</span>{/each}</pre>
						{#if diff.truncated}
							<div class="cv-notice">Diff truncated at 1 MB.</div>
						{/if}
					{/if}
				{/if}
			{/if}
		</section>
	</div>
</div>

{#snippet renderNode(node: TreeNode, depth: number)}
	{#if depth > 0}
		{@const isOpen = !collapsed[node.path]}
		<div
			class="cv-row"
			class:cv-dir={node.isDir}
			style:padding-left="{depth * 12}px"
			role="button"
			tabindex="0"
			onclick={() => (node.isDir ? toggleCollapse(node.path) : loadPath(node.path))}
			onkeydown={(e) => {
				if (e.key === 'Enter' || e.key === ' ') {
					e.preventDefault();
					node.isDir ? toggleCollapse(node.path) : loadPath(node.path);
				}
			}}
			aria-pressed={!node.isDir && selected === node.path ? 'true' : undefined}
		>
			{#if node.isDir}
				<span class="cv-caret">{isOpen ? '▾' : '▸'}</span>
				<span class="cv-name">{node.name}</span>
				{#if dirtyFolder(node)}
					<span class="cv-dot dot-modified" title="contains changes"></span>
				{/if}
			{:else}
				<span class="cv-caret-spacer"></span>
				<span
					class="cv-name cv-file-name"
					class:cv-selected={selected === node.path}
				>
					{node.name}
				</span>
				{#if node.status && node.status !== 'clean'}
					<span
						class="cv-dot {statusDotClass(node.status)}"
						title={statusTitle(node.status)}
					></span>
				{/if}
			{/if}
		</div>
	{/if}
	{#if node.isDir && (depth === 0 || !collapsed[node.path])}
		{#each node.children as child (child.path)}
			{@render renderNode(child, depth + 1)}
		{/each}
	{/if}
{/snippet}

<style>
	.code-viewer {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		background: var(--bg-surface, var(--bg));
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
		font-size: 0.8rem;
	}

	.cv-header {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 10px;
		border-bottom: 1px solid var(--border);
		background: var(--bg);
		flex-shrink: 0;
	}

	.cv-title {
		font-weight: 600;
		color: var(--text);
	}

	.cv-branch {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.cv-count {
		font-size: 0.72rem;
		color: var(--text-muted);
		margin-left: auto;
	}

	.cv-refresh {
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.8rem;
		padding: 2px 8px;
	}

	.cv-refresh:disabled {
		opacity: 0.5;
	}

	.cv-refresh:hover:not(:disabled) {
		color: var(--text);
	}

	.cv-body {
		display: grid;
		grid-template-columns: 240px 1fr;
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}

	.cv-tree {
		overflow-y: auto;
		border-right: 1px solid var(--border);
		padding: 4px 0;
		background: var(--bg);
	}

	.cv-pane {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}

	.cv-pane-header {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 10px;
		border-bottom: 1px solid var(--border);
		background: var(--bg);
		flex-shrink: 0;
	}

	.cv-pane-path {
		font-size: 0.75rem;
		color: var(--text);
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.cv-mode-toggle {
		display: flex;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.cv-mode-toggle button {
		background: transparent;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.72rem;
		padding: 3px 8px;
	}

	.cv-mode-toggle button.active {
		background: var(--accent, #7c3aed);
		color: #fff;
	}

	.cv-mode-toggle button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.cv-row {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 2px 8px 2px 0;
		cursor: pointer;
		user-select: none;
		color: var(--text);
		white-space: nowrap;
	}

	.cv-row:hover {
		background: rgba(255, 255, 255, 0.05);
	}

	.cv-caret {
		width: 10px;
		color: var(--text-muted);
		font-size: 0.7rem;
	}

	.cv-caret-spacer {
		width: 10px;
	}

	.cv-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
	}

	.cv-file-name.cv-selected {
		color: var(--accent, #7c3aed);
		font-weight: 600;
	}

	.cv-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.cv-dot.dot-modified,
	.cv-dot.dot-renamed {
		background: #ff9800;
	}

	.cv-dot.dot-added,
	.cv-dot.dot-untracked {
		background: #4caf50;
	}

	.cv-dot.dot-deleted {
		background: #f44336;
	}

	.cv-file,
	.cv-diff {
		margin: 0;
		padding: 8px 10px;
		flex: 1;
		overflow: auto;
		font-size: 0.78rem;
		line-height: 1.45;
		background: var(--bg);
		color: var(--text);
		white-space: pre;
	}

	.cv-diff :global(.diff-add) {
		color: #4caf50;
	}

	.cv-diff :global(.diff-del) {
		color: #f44336;
	}

	.cv-diff :global(.diff-hunk) {
		color: #7aa2f7;
	}

	.cv-diff :global(.diff-meta) {
		color: var(--text-muted);
	}

	.cv-placeholder,
	.cv-loading {
		padding: 14px;
		color: var(--text-muted);
		font-size: 0.8rem;
	}

	.cv-error {
		padding: 14px;
		color: #f44336;
		font-size: 0.8rem;
	}

	.cv-notice {
		padding: 6px 10px;
		border-top: 1px solid var(--border);
		color: var(--text-muted);
		font-size: 0.72rem;
		flex-shrink: 0;
	}

	.mono {
		font-family: var(--font-mono);
	}
</style>
