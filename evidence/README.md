# evidence/

One directory per run: `evidence/<run-id>/`. Run directories are gitignored;
`manifest.schema.json` and this file are tracked.

Contents (per the demo plan, "Evidence pack"):

| File | Producer | Contents |
|---|---|---|
| `manifest.json` | `swf-cli` | Source base, patch identity, executable digest, simulator/platform, policy, scope |
| `events-<scenario>.jsonl` | `swf-app` session | Every protocol message both directions + session records |
| `scenario-results.json` | `swf-app` session | Per-scenario outcome, dispatches, rejections, ticks, wall time |
| `build-metrics.jsonl` | runner scripts | Cold/warm build wall times, cache namespace, IB flag |
| `agent-context/` | `cold.sh` | Work order, base revision, seeded source given to the agent |
| `artifact/` | `warm.sh` | The actual exported executable + sha256 |
| `generated-scenarios/` | `swf-cli matrix` | Concrete scenario JSONs expanded from the coverage matrix |

Scope statement (also in every manifest): Rust contract checks, bridge
checks and simulated robot scenarios. Hardware HIL and physical validation
are not performed. A digest identifies an artifact; it is not by itself
proof of correct behavior — verdicts come from the protected verifier
(`demo/robot-sim/acceptance/verify_run.py`) bound to the actual artifact.
