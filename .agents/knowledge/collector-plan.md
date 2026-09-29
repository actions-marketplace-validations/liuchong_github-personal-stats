# Collector plan

Decided 2026-09-29. This is the work list for the local collector and the
daemon. It does not change the version and it does not publish anything.

The scheduled scan is a child process, and the terminal session index decides
which session files are opened. What remains is the readers below.

## What is installed now

The three binaries in `~/.local/bin` are Mach-O x86_64. The machine is arm64,
and the compiler on it is `aarch64-apple-darwin`. Launchd runs
`github-personal-stats-daemon serve` under the label
`dev.liuchong.github-personal-stats`. The state directory is
`~/.local/state/github-personal-stats`. The state config selects the git sink,
so the published days live in that directory's `storage/snapshots/`, one file
per machine per day. The `--output` path in the service file is unused while
the config names a sink.

`rebuild_on_schedule` calls `collect` in the daemon process, then sleeps thirty
minutes, and the loop collects once before the first sleep. A scan's working
set is the whole transcript tree. macOS does not return that memory, so the
process that scanned once stays large until it exits. On 2026-09-29 the daemon
had been up twenty-five days, was using no CPU, and was still about 580 MB
resident, with a virtual size of tens of gigabytes.

## Memory and CPU

Two changes, in this order, and the running process is not replaced until both
exist:

1. The timer starts `github-personal-stats-collect` and waits for it to exit.
   The daemon's own resident size is then the panel and the pulse listener.
   The scan's pages die with the child. This is the fix for the number that
   stays high while the process is idle.
2. A scan does not reread a store it has already finished. Resume positions
   live in the state directory, next to the machine id, and they are not
   published. The first one is the terminal index that already records
   `updated_at_ms`, so the large transcript tree is opened only for sessions
   that moved. Later readers use a path, a size, and an offset.

Native `aarch64-apple-darwin` is how both binaries are built and how they are
installed into `~/.local/bin`. That removes the translation on every scan. It
is not, by itself, a smaller idle process.

Acceptance for the swap: `file` reports arm64 for the installed daemon and the
installed collector. After launchd reloads, the daemon's resident size before
the child finishes is the size of the server, and it is still that size after
the child has exited. A missing source directory does not fail the run.

## Sources

Time is one timeline. Tokens are added once per spend. A second store that
describes the same call is not a source.

Already read: Cursor's code-tracking database, the terminal transcript tree,
and Claude's project transcripts. Editor presence stays the pulse journal and
is not added to agent time.

Add, in this order, each behind fixtures and the existing line cap:

1. Pi. Session JSONL, usage on the assistant message, same folding as Claude.
2. OpenCode. The session table only: the token columns are cumulative, so
   store the last total and record the growth, on the day of `time_updated`.
   Do not parse message bodies.
3. che. Session JSONL. A line over the cap still has to yield the timestamp
   and the provider input and output on the context snapshot, without holding
   the whole line. Dedupe by snapshot revision. The index is not a ledger.
4. DeepSeek Harness. Its JSONL plugin requires an explicit root. Read
   `session.jsonl` under that root, including the root a launcher set. Also
   read `~/.dsh` when a root appears there. Usage is the assistant chunk.
5. The launcher's own agent. Usage events whose source tool is that agent, in
   `canonical-events.jsonl`. Do not read routing files, environment snapshots,
   or journal lines for a tool that has its own reader.
6. Kimi and Kimi Code, after a count of wire event types and payload keys.
   Timestamps are there. Token fields were not found in the samples already
   read, so do not invent a usage shape.
7. Gemini and OpenClaw. An empty directory or an empty table returns nothing
   and is not an error. Record a moment when a timestamp exists. Do not invent
   tokens. Do not open tables that hold credentials.

A local reviewer that persists nothing is not a reader. Do not scrape review
comments to reconstruct a run.

## Tests

Each new reader gets a fixture that is a few lines, including one line over
the cap where the reader claims to handle that. The OpenCode fixture proves a
second observation of the same cumulative total adds nothing. The launcher
fixture proves an external tool's usage event is ignored and the built-in
agent's is kept. The daemon test proves the scheduled rebuild is a child
process. Coverage stays on the workspace gate.
