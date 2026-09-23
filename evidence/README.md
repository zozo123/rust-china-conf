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

The controlled EC2/Incredibuild demonstration also creates:

| Path | Producer | Contents |
|---|---|---|
| `<run-id>-build/build-proof/receipt.json` | Rust `swf-cli` | Normalized native, IB-cold and IB-parent-warm samples |
| `<run-id>-build/build-proof/summary.txt` | Rust `swf-cli` | Fail-closed verdict, medians and ranges |
| `<run-id>-build/build-proof/raw/` | benchmark harness | IB console, Build History and cache-statistics responses |
| `<run-id>-e2e/e2e-receipt.json` | E2E wrapper | Build and behavior evidence links, phase timings and scope |

The proof receipt is valid only when every IB sample contains positive remote
task and remote-core-time counters, every cold sample has zero cache hits, every
parent-warmed sample has positive cache hits, and each mode has at least five
samples from the same candidate. Missing or ambiguous telemetry fails the run.

Scope statement (also in every manifest): Rust contract checks, bridge
checks and simulated robot scenarios. Hardware HIL and physical validation
are not performed. A digest identifies an artifact; it is not by itself
proof of correct behavior — verdicts come from the protected verifier
(`demo/robot-sim/acceptance/verify_run.py`) bound to the actual artifact.
