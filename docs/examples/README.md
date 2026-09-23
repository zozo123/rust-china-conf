# Committed example evidence

Captured from a full local rehearsal (`scripts/robot-demo/rehearse.sh`,
`ROBOT_DEMO_BACKEND=robosuite`) on CPython 3.12 / robosuite 1.5.2 /
MuJoCo 3.9.0, macOS arm64:

- `sil-runner-a/` — **failure record** from the seeded revision: the stale
  (600 ms) episode is wrongly permitted (4 task dispatches), plus the agent
  context packet exported before the runner was destroyed.
- `sil-runner-b/` — **validated fixed candidate**: fresh isolated runner,
  `demo/fallback-patch.diff` applied to the exact base, contract suite
  10/10, 17-episode coverage matrix, protected verifier **88/88 PASS**.

The exported executable binary itself is not committed (size); its sha256
is recorded in `sil-runner-b/artifact/swf-cli.sha256` and in the manifest.
Regenerate a verifiable artifact any time with `scripts/robot-demo/rehearse.sh`.

Scope reminder: these are Rust contract checks, bridge checks and simulated
robot scenarios. Hardware HIL and physical validation were not performed.
