---
marp: true
title: A Million Compiles. One Robot Hour.
paginate: true
---

# A Million Compiles. One Robot Hour.

**Disposable Runners, Warm Cargo Factory**

Burn the runner. Keep the proof. Spare the robot.

---

## The stakes

- Your team has **one hour** booked on a robot tomorrow
- Agents produce **many** candidate changes overnight
- Which change deserves that hour?

> Today the robot is simulated, so we can repeat failures cheaply.

---

## The task (deliberately boring)

- Simulated Panda arm, one cube, robosuite `Lift`
- Success = the environment's own check + explicit height condition
- The interesting part is **around** the arm:
  who authorizes motion, on what information, built by which runner

---

## The safety gate (pure Rust)

1. Simulated emergency stop beats **everything**
2. Future/invalid observation timestamps → reject
3. Observation age ≤ 250 ms at dispatch → permit, else **StalePerception**

*250 ms is an illustrative demo policy, not a safe hardware threshold.*

---

## The seeded defect

- The freshness check is missing (conference fixture, labeled)
- Observation stream delayed 600 ms — real hold steps, real simulated time
- Gate permits every pickup segment anyway; the cube even lifts
- **Only the protected assertion catches it:** a stale episode must have
  zero task-action dispatches

---

## Architecture

agents → disposable runners → warm Cargo factory **[IB]** → retained
evidence → simulation → *future HIL / robot gate (not performed)*

- IB accelerates **compilation only**
- Tests and simulator checks always re-execute
- Verdicts are never restored from cache

---

## DEMO (protected six minutes)

runbook: `docs/talk/runbook-6min.md` — automation: `scripts/robot-demo/rehearse.sh`

---

## "The runner is gone."

- Runner A: fresh worktree, empty outputs, empty cache namespace
- Export: patch, base revision, context packet
- **Destroy A.**

> The change, the investigation, and the reusable compilation work survive.

---

## Runner B: fresh machine, warm factory

- New instance, same base image, fresh filesystem
- Patch applied to the **exact** base revision
- Compilation reuse from the parent-warmed cache
- Protected checks re-run against the actual built executable

---

## Behavioral payoff

> It refuses the stale request. With current information, it completes the task.

- stale episode → `rejected_stale`, **0** task dispatches
- fresh episode → `cube_lifted`
- e-stop → `emergency_stop` · dead bridge → explicit `timeout`

---

## Measurements (controlled)

- Same fixed source revision in every comparable row
- Native cargo baseline / IB cold / IB warm-from-parent
- ≥5 runs per mode; medians **and** ranges; run order disclosed
- Cache hit rate names its denominator
- Wall-time ≠ CPU-hours ≠ cost ≠ robot hours — unless measured

---

## Evidence pack

`manifest.json` · `events-*.jsonl` · `scenario-results.json` ·
`build-metrics.jsonl` · `agent-context/` · exported artifact + sha256

A digest identifies an artifact; the **protected verifier** binds results
to the actual artifact.

---

## Scope honesty

Performed: Rust contract checks, bridge checks, simulated robot scenarios.
**Not** performed: hardware HIL, physical validation, trained vision
(cube pose is simulator state; camera feed is for the audience).

---

## Close

> Every candidate needs another check. It does not need every dependency
> compiled from scratch.

**Burn the runner. Keep the proof. Spare the robot.**

Repo: github.com/zozo123/rust-china-conf
