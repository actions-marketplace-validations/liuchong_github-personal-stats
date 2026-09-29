# Current State

## Date

2026-09-29

## Project State

Scheduled collection runs the collector as its own process, and that process exits. The terminal session index decides which session files are opened; a session that has not moved is replayed from the state directory. The panel reads the published record. The binaries for this machine are built for aarch64.

## Active Task

Add the readers named in `.agents/knowledge/collector-plan.md`, in that order, each with fixtures. Pi is first. Do not publish, and do not change the version.

## Next Safe Task After Commit

OpenCode's session table, recording the growth of its cumulative token columns rather than the totals again.

## Constraints

- Do not write private reference names, paths, URLs, or copied text into repository files.
- Keep business code free of explanatory comments.
- Keep each committed stage independently buildable and reviewable.
