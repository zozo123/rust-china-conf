---
marp: true
title: A Million Compiles. One Robot Hour.
paginate: true
---

[简体中文幻灯](slides.md)

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

Freshness check missing. Observation stream delayed 600 ms — real
historical observations, simulated time still advancing.

Four proposals, ticks **12 / 22 / 32 / 48**. All four **permitted**.

---

## And the episode passed

```
episode_end  success=true  reason=cube_lifted  cube=0.9985 m
```

The cube came up. The simulator said success. In a CI summary,
this run is **green**.

Only the protected assertion caught it:
*a stale episode must have zero task-action dispatches.* This one had four.

---

## Architecture

agents → disposable runners → warm Cargo factory **[IB]** → retained
evidence → simulation → *future HIL / robot gate (not performed)*

- IB accelerates **compilation only**
- Tests and simulator checks always re-execute
- Verdicts are never restored from cache

---

## Where this was supposed to run

Ephemeral sandboxes. Rent a runner for ninety seconds, throw it away.

- Asked for 8 vCPU — **over quota**
- Asked for 8192 MB memory — **over quota**
- Asked for 40 GB disk — **over quota**
- IB helper cache wanted 10 GB; sandbox had **9 GB** free
- Moved to Debian 12 — coordinator **does not support it**

> `insufficient credit balance to create a sandbox`

---

## So it ran somewhere else

A real Incredibuild 4.31.0 grid: coordinator + initiator + 2 helpers.

**Nothing in the architecture changed.**

A fresh worktree. A fresh `CARGO_TARGET_DIR`. A directory of evidence
files. A digest-bound verdict.

> If your validation loop only works on one vendor's sandbox, you don't
> have a disposable runner. You have a pet with a short life expectancy.

---

## DEMO (protected six minutes)

runbook: `docs/talk/runbook-6min.md` — automation: `scripts/robot-demo/rehearse.sh`

replay, offline: `docs/demo/index.html`

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

Rust tests: **10 / 10** (8 contract + 2 unit). On the seeded build, 3 failed.

---

## Behavioral payoff

| episode | result | dispatches |
|---|---|---|
| stale 600 ms | `rejected_stale` at tick 12 | **0** |
| fresh 0 ms | `cube_lifted`, 0.9978 m | 4 |
| e-stop | `emergency_stop` at tick 10 | 1 |
| dead bridge | explicit `timeout` | 0 |

17 episodes total: 10 lifted, 5 refused, 1 stopped, 1 timed out.

---

## What I measured

| phase | wall | distributed |
|---|---|---|
| cold (runner A) | 21426 ms | `ib: true` |
| warm (runner B) | 21948 ms | `ib: true` |

The warm build was **522 ms slower**.

---

## Which is not a speedup

Different runners, different phases, **one run each**, a workspace small
enough that distribution overhead plausibly exceeds the work distributed.

What these two numbers establish: the accelerated path really ran, on a
real grid, on both sides of destroying the runner. **Workflow continuity.**

That is all they establish.

---

## The benchmark I still owe you

- Same fixed source revision in every comparable row
- Native cargo baseline / IB cold / IB warm-from-parent
- **≥5 runs per mode**; medians *and* ranges; run order and contention disclosed
- Cache hit rate names its denominator
- Separately: runner startup, checkout, artifact transfer, agent latency

Wall-time ≠ CPU-hours ≠ cost ≠ robot hours — unless measured.

---

## Evidence pack

`manifest.json` · `events-*.jsonl` · `scenario-results.json` ·
`build-metrics.jsonl` · `agent-context/` · exported artifact + sha256

**88 / 88** protected checks, bound to
`f358e898b4e4f0cf00c840de314c9e7c42cb246a4b97f1abffd3071fbecc8326`

A digest identifies an artifact; the **protected verifier** binds results
to the actual artifact.

---

## Scope honesty

Performed: Rust contract checks, bridge checks, simulated robot scenarios.

**Not** performed: hardware HIL, physical validation, trained vision
(cube pose is simulator state; camera feed is for the audience).
No speedup measured.

> An evidence pack whose boundary you can't see isn't evidence.
> It's a green checkmark with better production values.

---

## Close

> Every candidate needs another check. It does not need every dependency
> compiled from scratch.

The seeded run in this demo was green. That's the whole lesson.

**Burn the runner. Keep the proof. Spare the robot.**

Repo: github.com/zozo123/rust-china-conf
