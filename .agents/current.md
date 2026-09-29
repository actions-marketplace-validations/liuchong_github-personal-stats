# Current State

## Date

2026-09-29

## Project State

The renderer, the collector, and the daemon are in use on this machine. The installed binaries are x86_64. The daemon has been up for weeks and is holding the high-water mark of an in-process scan.

## Active Task

Follow `.agents/knowledge/collector-plan.md`. First slice: build the collector and the daemon for aarch64-apple-darwin, and move the scheduled scan into a child process that exits, before replacing the running daemon. Do not publish, and do not change the version.

## Next Safe Task After Commit

Add the readers in the order that plan names, each with fixtures, and keep external tools on their own stores.

## Constraints

- Do not write private reference names, paths, URLs, or copied text into repository files.
- Keep business code free of explanatory comments.
- Keep each committed stage independently buildable and reviewable.
