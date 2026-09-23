# A Million Compiles. One Robot Hour.

Final implementation and conference plan — 23 September 2026

Technical subtitle: **Disposable Runners, Warm Cargo Factory**

Mantra: **Burn the runner. Keep the proof. Spare the robot.**

## 1. Scope and current status

Build one end-to-end **software-in-the-loop (SIL)** demonstration. A simulated Panda arm must lift a cube. A deliberately seeded bug in a Rust supervisor permits pickup actions from stale observations. A coding agent repairs that bug. A new Linux runner compiles and checks the candidate through Incredibuild. The actual resulting executable rejects the stale case and completes the fresh case in the simulator.

The user selected a simulator. No physical controller or robot participates in this version, so do not label its results hardware-in-the-loop validation. A human approval button is also not hardware HIL. Physical hardware remains the downstream motivation for catching failures early.

The factory repository dependency inventory has been audited, and the candidate simulator environment has been resolved from package metadata. The robot integration, graphical runtime, scripted controller, IB integration and benchmark have **not** been implemented or validated in this task. This document specifies that work.

Use the existing factory repository snapshot `59339a3dc9d66403d95b3a2dd2129df90bbd3cc4` as the starting reference, then pin a new demo commit once implemented. The existing repository is orchestration and operator tooling; it does not already provide this robot integration.

## 2. The audience story

1. Show a simulated arm and one cube: a visible job with a clear success condition.
2. Delay the observation stream. The seeded Rust bug authorizes a pickup from old information; a protected assertion catches the defect.
3. Give the agent the failure trace, the contract, and a narrow edit scope.
4. Export the patch and context. Destroy the runner that produced them.
5. A fresh Linux runner reuses eligible compilation work through IB and executes the checks again.
6. The built executable blocks stale pickup requests. A fresh-observation episode lifts the cube.
7. Retain the source/artifact identities and observed results. Explain that hardware validation is still a separate downstream stage.

Opening passage:

> Imagine your team has one hour booked on a robot tomorrow. Agents can produce many proposed changes overnight. Which change deserves that hour? Today the robot is simulated, so we can repeat failures cheaply. We will repair a Rust defect, destroy the machine that produced the patch, and validate the change on a fresh Linux runner using Incredibuild.

Central line, delivered when Runner A disappears:

> The runner is gone. The change, the investigation, and the reusable compilation work survive.

Behavioral payoff:

> It refuses the stale request. With current information, it completes the task.

Closing passage:

> Every candidate needs another check. It does not need every dependency compiled from scratch. Incredibuild accelerates that Rust build loop. The simulator lets us test this behavior before spending hardware time. Burn the runner. Keep the proof. Spare the robot.

The title is a statement about scale and priorities. Do not imply a measured conversion between a million compilations and one robot hour.

## 3. Fixed task and environment

- Simulator framework: robosuite 1.5.2.
- Physics engine: MuJoCo 3.9.0, a candidate pin below the documented robosuite incompatibility boundary at MuJoCo 3.10.
- Simulator environment: a separate CPython 3.12 environment on Linux x86-64.
- Robot: simulated Panda with its configured gripper.
- Task: robosuite `Lift`, one cube on a table.
- Controller: start from the pinned Panda BASIC controller configuration, using its operational-space arm control. Inspect and pin its actual action dimensions and scaling before authoring actions.
- Motion generation: a rehearsed scripted approach, grasp and lift sequence. No model training in the live path.
- Observation source: simulator-provided object/robot state, explicitly described as simulated observations. Rendered camera footage is for presentation; do not imply that a trained vision system detects the cube.
- Display: one fixed camera; stable cube placement for the live episode.
- Validation: use a defined set of placements and delays in preparation, and report those cases. A fixed seed supports replay within the pinned environment; do not promise bit-identical results across different platforms.

Use the environment's actual success check plus explicit height/episode conditions agreed during implementation. Do not equate episode termination with success: robosuite environments can continue until their configured horizon after reaching a goal.

Source: [robosuite environments](https://robosuite.ai/docs/modules/environments.html), [controllers](https://robosuite.ai/docs/modules/controllers.html), [Lift source at v1.5.2](https://github.com/ARISE-Initiative/robosuite/blob/v1.5.2/robosuite/environments/manipulation/lift.py).

## 4. Architecture and ownership

The coding agent edits source in a disposable build runner. The factory coordinates the attempt and stores its artifacts. IB sits only in the compilation stage. The simulator runs on a separate, already prepared Linux host or process environment.

The candidate Rust executable and Python bridge execute on the same simulation host for the initial implementation. They exchange line-delimited JSON over process input/output. This avoids putting timing-sensitive simulation coordination across the conference network.

The simulator's live video can be displayed remotely, but display latency must not enter the simulated clock or the Rust build timing.

Proposed code additions, not existing commands or files:

```text
rust/crates/robot-safety-gate/
  src/lib.rs
  tests/contract.rs

rust/crates/swf-app/
  ... robot-demo session orchestration

rust/crates/swf-cli/
  ... robot-demo command routing

demo/robot-sim/
  bridge.py
  scripted_controller.py
  requirements-linux.txt
  config/
  scenarios/
  acceptance/

scripts/robot-demo/
  preflight.sh
  cold.sh
  warm.sh
  validate.sh
  rehearse.sh

evidence/<run-id>/
```

Responsibilities:

| Component | Owns |
|---|---|
| `robot-safety-gate` | Pure Rust decision contract; controlled clock inputs; typed rejection reasons |
| Rust application/session layer | Scenario identity, lifecycle, subprocess protocol, decision logging, timeouts |
| Python bridge | MuJoCo/robosuite state, proposal generation, authorization enforcement, stepping and observed outcomes |
| Scripted controller | Rehearsed motion proposals from simulated state |
| Protected verifier | Acceptance fixtures, diff restrictions, trace assertions, artifact identity checks |
| Factory | Agent attempt lifecycle, source/patch/context retention, runner cleanup |
| Incredibuild | Supported compilation caching and distribution |

The CLI and application layers must actually perform these jobs. Their dependency graph is a legitimate measured workload only when the built executable is used in the demonstration.

Keep a direct scripted factory route for stage operation. Airflow may schedule it backstage; the audience should not need an Airflow walkthrough to understand the result.

## 5. Rust policy and seeded defect

Contract:

- Check the simulated emergency-stop input first.
- Reject invalid or future observation timestamps.
- Permit task actions when observation age is at most 250 ms.
- Reject task actions when observation age exceeds 250 ms.
- Evaluate freshness at the final dispatch decision.
- Rejection prevents new pickup/task actions; the simulation may continue using an explicit hold behavior.

The 250 ms threshold is an illustrative demo policy, not an established safe threshold for physical robots.

Seed only the omitted freshness check. Keep stop handling, invalid timestamp handling, threshold configuration, transport validation and protected tests unchanged. Label the regression as a conference fixture in both README and issue text.

Agent work order:

> Fix the seeded stale-observation regression in robot-safety-gate. At dispatch, observations older than 250 ms must return StalePerception. Preserve the simulated emergency-stop precedence and timestamp validation. Preserve boundary behavior at 250 and 251 ms. Do not edit simulator code, fixtures, the threshold, acceptance checks, runner scripts or evidence generation. Return the patch and relevant test output.

Enforce the allowed source/test paths through a diff validator. Use a protected verifier copy of the acceptance suite rather than trusting agent-authored reports.

## 6. Timing and action protocol

Use one controlled simulation time domain for the task contract. Wall-clock build/model/display times are separate measurements.

Each proposal includes at least:

```text
run_id
episode_id
action_id
simulation_tick
simulation_time_ns
observation_id
observation_capture_time_ns
simulated_stop
proposed_action
```

The Rust decision echoes the run, episode, action and tick identifiers and includes the decision/reason.

Protocol invariants:

1. Only one task action is outstanding at a time.
2. Python does not advance the simulated state between the final Rust decision and applying that matching action.
3. Approvals are valid only for the exact action and tick; old, duplicate or mismatched approvals cannot dispatch a task action.
4. A missing, malformed or timed-out response leads to hold/episode failure, not default authorization.
5. A rejection produces no pickup/task-action dispatch. Explicit hold steps are separately labeled and may still advance physics.
6. Any queue delay happens before the final dispatch decision, and the proposal includes the updated simulation time.

Return to the Rust gate between every guarded simulator control step. A single approval must not authorize an entire trajectory if stop handling is demonstrated during motion. Keep protocol messages alone on standard output and all simulator diagnostics on standard error.

Inject staleness by delivering an actual historical observation while simulation time progresses through hold steps. Sleeping while the simulation is paused does not make a simulated observation older.

For the stage fixture, choose a convenient exact delay such as 600 ms at the configured control rate; do not force arbitrary age values that the modeled observation schedule cannot produce. Test 250/251 ms exactly in the pure Rust contract suite.

The trace must connect observation -> Rust decision -> actual bridge dispatch/hold -> simulator state -> assertion.

## 7. Acceptance criteria

| Check | Required result |
|---|---|
| Age 0, 50, 250 ms; no stop | Permit |
| Age 251 and 600 ms; no stop | StalePerception |
| Stop with fresh data | EmergencyStop |
| Stop with stale data | EmergencyStop |
| Future/invalid timestamp | Reject |
| Fresh observation ages beyond limit in queue | Reject at final dispatch |
| Old or mismatched action approval | No task-action dispatch |
| Rust process exits or protocol times out | Hold/episode fails explicitly |
| Stale simulated episode | No new pickup dispatches; rejection recorded |
| Fresh simulated episode | Cube-lift success condition reached |
| Same candidate rerun in protected suite | Fresh check results bound to that candidate |

Unit tests exercise exact timing boundaries. Integration checks exercise the real compiled Rust program and the real Python bridge. Simulator episodes exercise actual environment transitions. Keep the three result types visible in the evidence.

Prepare a small explicit coverage matrix, for example five supported cube placements times three freshness scenarios, plus stop and protocol-failure cases. Select only reachable placements the scripted controller can handle. Do not multiply identical episodes to make a coverage number look large.

## 8. Two-runner lifecycle

Runner A starts from the pinned base image with a fresh writable filesystem and empty configured compilation-output locations. It builds the seeded-bug revision through IB, executes the failing acceptance case, and populates a controlled shared compilation-cache namespace.

Give the failure trace and relevant source context to the agent. Export its patch, source identity and concise context packet before destroying A. If the agent runs exploratory builds, use a separate IB cache namespace or disable their shared-cache writes so they do not prewarm the measured fixed candidate.

The context packet includes the work order, base revision, patch, allowed paths, failing assertion, acceptance criteria, validation command and remaining time/attempt budget.

Runner B uses the same base image but a new instance and fresh writable filesystem. Apply the exported patch to the exact base. Record the resulting source tree identity and build configuration. Build through the parent-warmed IB cache and execute the protected checks anew.

Export the actual executable, its digest, logs and evidence. The simulation host verifies the executable digest and runs that artifact. Use compatible Linux architecture and runtime libraries between runner and simulation host. Export any required runtime libraries/configuration together with the artifact; do not recompile a different executable silently on the display machine.

Retain the evidence and destroy B after the demonstrated run/export. Explicitly acknowledge that patches, logs, the base image and external cache persist; do not say literally nothing survives.

## 9. Workload and IB configuration

The measured workload must compile the Rust executable that participates in the simulation. Proposed Cargo work includes building `swf-cli` and compiling the new gate/application acceptance tests. Validate exact package selections against the implemented workspace before publishing commands.

Suggested logical workload:

```text
cargo build -p swf-cli --locked
cargo test -p robot-safety-gate -p swf-app --locked --no-run
execute the resulting required test binaries
```

This is a proposed workload, not a verified IB CLI invocation. The IB engineering owner must supply and validate the supported Linux wrapper/configuration. Capture Cargo test artifact metadata so the verifier runs the correct test executables outside result-caching boundaries; include any other required checks explicitly.

Only compilation/artifact reuse is accelerated in the demo claim. Acceptance tests and simulation checks must execute again. Verify that the IB profile does not cache and replay their outcomes.

Pin toolchain, lockfile, target triple, profile, flags, Cargo configuration and path normalization. Check all configured target/intermediate output paths. Keep dependency downloads outside timed compilation, equally provisioned on all runners. Exclude other compiler caches or restored build outputs that would obscure attribution.

Native dependencies may appear in a Rust application's build graph. Report Rust compiler, native compiler and linker activity separately when the telemetry supports it. Show distributed work only when it actually executes on helpers. Do not imply distribution of a serial Rust compiler operation merely because helpers are configured.

## 10. Performance experiment

Benchmark the same final fixed source revision in all comparable rows. The live broken-to-fixed sequence establishes workflow continuity but is not a controlled same-source speed comparison.

| Mode | Cache state | Purpose |
|---|---|---|
| Native Cargo on fresh runner | No compilation cache | Baseline validation time |
| IB local execution control | Cache disabled; helpers disabled | Optional isolation of wrapper overhead/local execution |
| IB with available helpers | Empty compilation cache | Measure cold accelerated configuration |
| IB with same helper configuration | Populated only from the parent revision | Measure reuse after the demonstrated edit |

Use a minimum of five runs per reported mode. Reset isolated cache state for each cold run; re-seed parent-only state for each edited warm run. Record run order and resource contention. Use matched resources and disclose helper capacity, parallel-job settings, and the native baseline configuration. Do not run competing benchmarks on shared helpers unless contention is part of the defined experiment.

Report medians and ranges for compilation/linking, fresh tests, simulator checks, and total validation. Separately report runner startup, checkout, artifact transfer and agent response time. Cache hit percentage must name its denominator.

The 46 s cold / 13 s warm bands remain goals until measured. A higher hit rate is diagnostic; the primary outcome is useful validation latency. Do not equate wall-time savings with CPU-hours, cloud-cost savings, model speedup or hardware-hour savings without measuring the relevant quantity.

First feasibility gate: the actual workload must show a material, repeatable benefit. If not, revisit the real product workload before producing polished graphics; never add unused dependencies or artificial waits.

## 11. Six-minute stage runbook

| Time | Stage action |
|---|---|
| 0:00–0:30 | Show Panda/cube and introduce the stale observation |
| 0:30–1:15 | Runner A baseline build/failing assertion; explain the exact policy |
| 1:15–2:05 | Agent produces bounded patch from failure context |
| 2:05–2:30 | Show small diff, export context, destroy A |
| 2:30–3:15 | Fresh B builds through IB and runs checks |
| 3:15–4:00 | Patched executable rejects stale episode; show actual zero pickup dispatches |
| 4:00–4:40 | Fresh episode lifts cube; show task success |
| 4:40–5:20 | Evidence and measured build comparison |
| 5:20–5:40 | Explain simulation scope and downstream hardware gate |
| 5:40–6:00 | Buffer |

These allocations are rehearsal budgets, not claimed runtimes. If the cold build does not fit, show a labeled recording or measured prior result for that phase and retain the fresh warm build and simulated behavior live. Keep one complete six-minute recording available locally.

Cap the live agent attempt at roughly 45 seconds; visibly apply the reviewed fallback patch if necessary. No silent substitution of scripted agent output, cached test verdicts or recorded timings for live results.

Prepare the simulator and assets before the talk. Start the measured runner instances only when their phases begin. Use the prepared recording if network or rendering infrastructure fails rather than debugging live.

## 12. Presentation view

Use the simulator as the main visual, with a restrained adjacent terminal view. A new web dashboard is unnecessary.

Show only:

- Robot/cube scene and current episode type.
- Observation age, policy decision and task-action dispatch count.
- Runner identity and clean-output confirmation.
- Actual IB cache/remote/local work and elapsed time.
- Candidate identity and test/scenario result.

Colors should correspond to actual states: failure, hold/rejection, and successful task completion. The rendering must follow actual simulation state, not a separate animation.

Use the agreed single architecture diagram in the talk: agents -> disposable runners -> warm Cargo factory [IB] -> retained evidence -> simulation -> future HIL/robot gate. Label the final hardware stages as not performed.

## 13. Talk structure and timing

25-minute version:

| Time | Content |
|---|---|
| 0–3 | Robot-hour stakes; explain today's simulated task |
| 3–7 | Agent demand: candidate changes, failed attempts, feedback and context |
| 7–10 | Linux Rust build graph, distribution, cache and clean runners |
| 10–16 | Protected six-minute demonstration |
| 16–21 | Measurements and whole-loop economics |
| 21–25 | Evidence boundary, downstream hardware validation, close |

20-minute cut: 2-minute opening, 3 minutes on agent loops, 3 minutes on IB, the same 6-minute demo, 4 minutes of results, 2-minute close.

The published 15:15–15:45 slot is 30 minutes. It supports 25+5 or 20+10, not 25+10. Confirm organizer timing; both talk cuts preserve the demo.

Keep robotics as the recurring visual and downstream consequence. Keep IB's claim centered on the build stage. Keep the existing IDE talk as a brief neighboring use case rather than reproducing it. Avoid a broader factory-philosophy or evidence-chain talk that would obscure the measured acceleration.

## 14. Implementation schedule and owners

The roles may be held by one or more people; names require team assignment.

| Deadline | Owner role | Deliverable and exit criterion |
|---|---|---|
| Sep 23–25 | Simulation engineer | Install candidate environment; Panda Lift renders and completes 20 consecutive fresh-observation episodes without intervention |
| Sep 23–27 | IB/Linux engineer | Pin supported IB/toolchain/image; prove actual Rust compilation reuse across two fresh runners; inspect helper work |
| Sep 26–30 | Rust/integration engineer | Gate and lock-step bridge; stale/fresh/stop/timeout cases connected to actual dispatch |
| Oct 1–4 | Factory engineer | Agent context packet, diff restrictions, runner lifecycle, export and artifact identity verification |
| Oct 5–8 | Validation owner | Controlled benchmark matrix, scenario coverage, repeatability results and raw evidence |
| Oct 9–12 | Presenter/demo owner | Six-minute runbook, 20/25-minute narration, local fallback recording; freeze code and images |
| Oct 13–15 | Presenter and IB owner | Three consecutive rehearsals; readability/network/graphics checks |
| Oct 16 | Demo owner | Remeasure on final environment; freeze numbers and source/artifact identities |
| Oct 17 | Presenter | Run pinned demonstration; preserve evidence and recording |

Simulation feasibility and IB acceleration are independent early workstreams. Both must pass before presentation polish.

## 15. Evidence pack

Save under a unique run ID:

```text
manifest.json
agent-context.json
candidate.diff
build-metrics.json
ib-build-report.*
tests.json
events.jsonl
scenario-results.json
environment.json
demo-recording.*
```

The manifest links source base, candidate tree/commit, lockfile digest, compiler and image identifiers, IB configuration, runner IDs, tested executable digest, simulator/model configuration, seeds, check results, phase timings, and artifact locations.

A digest identifies an artifact; it is not by itself proof of correct behavior or trusted authorship. Results must be collected by the verifier and bound to the actual artifact used in the episode.

State the observed scope: Rust contract checks, bridge checks and simulated robot scenarios. Hardware HIL and physical validation are not performed. Keep trial counts and seeds explicit; do not invent validation levels.

## 16. Release checklist

- The fresh scripted episode succeeds across the defined rehearsal matrix.
- The seeded failure is reproducible and identified as a demo defect.
- The patch is bounded and the protected acceptance fixtures are unchanged.
- The simulator action path actually depends on the tested Rust executable.
- Historical observations age on the simulation clock; approvals are tied to exact ticks/actions.
- Hold steps are distinguished from pickup actions in assertions and logs.
- Runner B starts without prior compilation outputs in every configured location.
- Parent-cache reuse is observable, and final-candidate exploratory builds cannot contaminate its measurement.
- Tests and scenario assertions execute again; their verdicts are not restored from cache.
- All speedup claims use controlled same-candidate comparisons.
- The executed artifact matches the exported digest and declared runtime environment.
- Simulator/runtime/IB configurations and recordings are pinned.
- Three consecutive runs complete within 5:30, preserving 30 seconds of contingency inside the six-minute slot.
- The fallback is usable locally and visibly labeled when used.
- Final materials consistently say simulation/SIL and distinguish future hardware validation.

## 17. Dependency deliverables

- `dependency-audit.md`: existing Rust/Python inventory, proposed simulation layer and integration boundaries.
- `simulator-requirements-linux.txt`: candidate 29-package exact-version resolution for Linux x86-64 / Python 3.12; not yet runtime-tested or hash-locked.

Keep factory and simulator Python environments separate. No live RL training or model checkpoint is needed for the scripted task. The chosen coding-agent client and account, disposable-runner provider, IB services, system graphics stack and base images must be pinned/provisioned in addition to application libraries.
