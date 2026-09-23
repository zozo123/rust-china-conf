# Conference architecture and readiness

[中文 README](../README.md) · [English README](../README.en.md) · [Talk](talk/talk-25min.en.md)

This is the current implementation plan. The original design assumed remote
sandbox instances and per-control-step supervision; the implemented demo uses
local Git workspaces and authorization at motion-segment boundaries.

## The promise

A candidate must satisfy an independent contract against the executable that
actually runs. Temporary workspaces can disappear while the patch and evidence
remain. Incredibuild is the intended compilation distribution/reuse layer;
reuse and performance require their own measurements.

## Story and ownership

1. The cube lifts using 600 ms-old observations. Task success and policy
   compliance disagree.
2. A bounded implementation patch restores the illustrative 250 ms rule.
3. Export the patch and source identity; remove workspace A.
4. Build in B with fresh outputs, then execute the checks again.
5. The patched executable rejects stale input and completes the fresh task.

| Layer | Current implementation |
| --- | --- |
| Operator | Mac browser, local media, source and evidence |
| Runner | Detached Git worktree plus unique Cargo output directory on the Linux initiator |
| Agent | External candidate patch; reviewed fallback is the scripted default |
| Build | Cargo via `ib_console` when present; `REQUIRE_IB=1` forbids silent native fallback |
| Contract | Rust stop precedence, timestamp validity, freshness at segment dispatch |
| Simulation | CPython 3.12, robosuite 1.5.2, MuJoCo 3.9.0, Panda Lift |
| Protocol | JSON lines over subprocess pipes, one outstanding proposal |
| Verification | Protected scenario expectations, ordered traces, observed dispatch counts, executable digest |
| Retention | Evidence and exported binary outside the removed worktree |

EC2 supplies today's execution hosts. Islo is a planned runner-provider
integration, not part of the checked-in execution path. Worktree separation
does not provide VM/container security isolation.

## Timing and controller scope

Simulation ticks are 50 ms (20 Hz). Staleness advances simulation time while
delivering a historical observation. Rust receives a proposal before each of
four segments: approach, descend, grasp, lift. A permitted segment may execute
multiple controller steps. Stop injection occurs at a segment boundary.

Build durations and communication timeouts use wall time. The code does not
establish real-time deadlines or a physical emergency-stop system. Cube state
comes from the simulator; there is no learned vision or policy training.

## Current verification

`python3 scripts/robot-demo/check.py` validates the current checkout in a
temporary copy. The original seeded gate remains unchanged. It requires the
specific three seeded failures, rejects the resulting stale episode, applies
the allowlisted patch and verifies the patched workspace and 17-case mock matrix.

The verifier derives expectations from protected repository configuration.
Missing/unknown/duplicate scenarios, missing or malformed traces, inconsistent
dispatch counts and artifact mismatches must fail. A checksum identifies a
binary; it is not a signed execution attestation or proof of general correctness.

The original Linux evidence remains immutable. Its 10/10 and 88/88 results
describe the original revision and verifier, not this expanded suite.

## Before the next live Linux demonstration

- Run the complete current rehearsal on the IB initiator with `REQUIRE_IB=1`
  and `ROBOT_DEMO_BACKEND=robosuite`.
- Retain the executable, patch, run manifest, simulator traces, build logs and
  verifier output under a new run ID.
- Confirm the pinned controller and dependency environment initializes and
  renders reliably. The earlier run does not validate newly changed code.
- Capture helper activity and effective cache controls before claiming
  distribution or reuse. The scripts do not configure cache isolation.
- Rehearse three times under 5:30 for a six-minute demo, including fallback.
- Keep local media and the labeled recorded replay available without a network.

## Controlled performance experiment

The recorded phases were 21426 ms and 21948 ms. They are different candidates,
one observation each. They do not establish a speedup or controlled cold/warm
cache states.

Use the same fixed candidate in each comparison: native Cargo, IB with empty
cache, and IB with a cache populated only from the parent revision. Measure at
least five samples per mode, disclose order and contention, and report medians
and ranges. Keep dependency acquisition outside the timed build. Pin toolchain,
lockfile, flags, target and build profile. Record the supported cache reset /
prewarm procedure and its telemetry.

Measure provisioning, checkout, model response, compilation, tests, simulation
and export separately. Compilation savings are not automatically savings in
total validation time, cost, CPU-hours or physical robot time.

## Website and language

The static Chinese and English landing pages share one template and translation
dictionary. `scripts/site/build.py --check` catches drift; the coverage table is
derived from recorded results. The English route has real static metadata.
The replay remains a schematic rendering of recorded decisions with pause,
step and final-result controls. Actual simulator media is labeled separately.

## Readiness boundary

Local mock validation and browser checks can establish software correctness and
presentation usability. Conference sign-off additionally needs a fresh Linux
SIL rehearsal, a timed delivery and a tested recording fallback. Islo migration,
physical robotics validation and a performance claim remain separate work.
