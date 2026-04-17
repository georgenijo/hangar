# hangar — vendored claude-pipeline

This is a vendored copy of [claude-pipeline](https://github.com/georgenijo/claude-pipeline) adapted for hangar. It runs the Claude Code multi-agent pipeline (context → architect → review → build → test → fix) against a GitHub issue.

## Differences from upstream

- **Model pinning**: all `opus`/`sonnet` aliases replaced with explicit IDs:
  - `opus` → `claude-opus-4-7`
  - `sonnet` → `claude-sonnet-4-6`

Rationale: ensures the pipeline always uses the intended model versions regardless of Claude CLI alias drift.

## Usage

```bash
# Single issue
./pipeline.sh <issue-number> --project-dir /path/to/repo

# All open issues sequentially (skips on failure)
./batch.sh --all-open --project-dir /path/to/repo

# Specific issues
./batch.sh --issues 5,7,9 --project-dir /path/to/repo
```

## Typical run on the box

```bash
cd ~/Documents/hangar
./.pipeline/batch.sh --all-open --project-dir $PWD
```

Runs every open GitHub issue in order. Logs to `~/Documents/pipeline-logs/hangar/issue-<N>/`.

## Agents

| Agent | Model (default) | Role |
|---|---|---|
| `context-gatherer` | `claude-sonnet-4-6` | Read issue + repo, produce context.md + models.json |
| `architect` | `claude-opus-4-7` | Produce implementation plan |
| `senior-reviewer` | `claude-opus-4-7` | Approve or request plan revisions |
| `builder` | `claude-sonnet-4-6` | Implement the plan, commit |
| `tester` | `claude-sonnet-4-6` | Run tests, produce results JSON |
| `fixer` | `claude-sonnet-4-6` | Fix failing tests |

The context-gatherer overrides these per issue based on complexity classification (see `agents/context-gatherer.md`).

## Safety

- `--dangerously-skip-permissions` is used inside the pipeline (required for non-interactive runs)
- Pipeline creates a branch per issue, never commits directly to main
- PRs are not auto-created by pipeline — CI + merge policy handled via GitHub Actions

## Keeping upstream in sync

Occasionally pull improvements from upstream:

```bash
cd /tmp && rm -rf claude-pipeline && git clone https://github.com/georgenijo/claude-pipeline
diff -r .pipeline/ /tmp/claude-pipeline/ | less
```

Cherry-pick manually (preserve the model overrides).
