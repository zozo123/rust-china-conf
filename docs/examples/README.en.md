# Committed example evidence

[简体中文](README.md)

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

## Linux / Incredibuild grid run

`ec2-runner-a/` and `ec2-runner-b/` were captured from run
`ec2-e2e-20260923-160725` on an Ubuntu 20.04 x86-64 Initiator connected to
an Incredibuild 4.31.0 grid (one Coordinator and two Helpers):

- both Cargo phases executed under `ib_console` with the checked-in
  `rust/ib_profile.xml`;
- Runner A reproduced the seeded defect: a 600 ms stale episode dispatched
  four task actions;
- Runner B applied the reviewed patch to the exact base revision, passed all
  10 Rust checks, and completed the 17-episode robosuite matrix;
- the protected verifier passed **88/88 checks** against the exported
  executable identity.

Observed build-phase wall times were 21.426 s cold and 21.948 s warm. This
proves the Incredibuild integration path, but it does **not** demonstrate a
speedup: the warm observation was slightly slower. Do not present these two
workflow-continuity timings as a controlled performance benchmark. The
planned same-candidate, minimum-five-run benchmark remains separate.
