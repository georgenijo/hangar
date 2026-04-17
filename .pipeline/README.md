# hangar — vendored claude-pipeline

This is a vendored copy of [claude-pipeline](https://github.com/georgenijo/claude-pipeline) adapted for hangar. It runs the Claude Code multi-agent pipeline (context → architect → review → build → test → fix) against a GitHub issue.

## Differences from upstream

- None at the moment. Uses upstream aliases `opus` / `sonnet`, which Claude Code resolves to the latest standard-context variants (Opus 4.7, Sonnet 4.6 standard as of 2026-04).

> Note: earlier we pinned explicit IDs (`claude-opus-4-7`, `claude-sonnet-4-6`) but the `-4-6` / `-4-7` bare IDs resolve to 1M-context variants which require Extra Usage on the account. Aliases sidestep that gate while still pointing at the latest model family.

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
| `context-gatherer` | `sonnet` | Read issue + repo, produce context.md + models.json |
| `architect` | `opus` | Produce implementation plan |
| `senior-reviewer` | `opus` | Approve or request plan revisions |
| `builder` | `sonnet` | Implement the plan, commit |
| `tester` | `sonnet` | Run tests, produce results JSON |
| `fixer` | `sonnet` | Fix failing tests |

The context-gatherer can override these per issue based on complexity classification (see `agents/context-gatherer.md`).

## Safety

- `--dangerously-skip-permissions` is used inside the pipeline (required for non-interactive runs)
- Pipeline creates a branch per issue, never commits directly to main
- PRs are not auto-created by pipeline — CI + merge policy handled via GitHub Actions

## Keeping upstream in sync

```bash
cd /tmp && rm -rf claude-pipeline && git clone https://github.com/georgenijo/claude-pipeline
diff -r .pipeline/ /tmp/claude-pipeline/ | less
```
