# Six-minute stage runbook

Allocations are rehearsal budgets, not claimed runtimes. One complete
six-minute recording stays available locally; if network or rendering fails,
play the recording — visibly labeled — rather than debugging live.

Automation: `scripts/robot-demo/rehearse.sh <run-id>` performs the whole arc
(`ROBOT_DEMO_BACKEND=robosuite` for the real SIL path). On stage, run the
phases individually so narration stays in charge of pacing.

| Time | Stage action | Command / artifact |
|---|---|---|
| 0:00–0:30 | Show Panda/cube; introduce the stale observation | simulator view; `demo/robot-sim/config/scenarios/stale_600ms.json` |
| 0:30–1:15 | Runner A baseline build + failing assertion; explain the policy | `scripts/robot-demo/cold.sh <run>-runner-a` |
| 1:15–2:05 | Agent produces bounded patch from failure context (cap ≈45 s) | work order: `evidence/<run>-runner-a/agent-context/work-order.md` |
| 2:05–2:30 | Show the diff, export context, **destroy A** | `demo/fallback-patch.diff` (49 lines); cold.sh teardown |
| 2:30–3:15 | Fresh runner B builds through IB, runs checks | `scripts/robot-demo/warm.sh <run>-runner-b` |
| 3:15–4:00 | Patched executable rejects the stale episode; show **zero** dispatches | matrix row `center-f600` → `rejected_stale`, dispatches=0 |
| 4:00–4:40 | Fresh episode lifts the cube; show task success | matrix rows `*-f0` → `cube_lifted` |
| 4:40–5:20 | Evidence + measured build comparison | `evidence/<run>-runner-b/`, `build-metrics.jsonl` |
| 5:20–5:40 | Simulation scope; downstream hardware gate | scope statement in every manifest |
| 5:40–6:00 | Buffer | — |

## Hard rules for the live run

- The live agent attempt is capped at ~45 s. If it misses, apply the
  reviewed fallback patch **visibly**. No silent substitution of scripted
  agent output, cached test verdicts, or recorded timings for live results.
- If the cold build does not fit the budget on the day, show a labeled
  recording or a measured prior result for that phase; keep the warm build
  and the simulated behavior live.
- Runner B must start with no prior compilation outputs in any configured
  location. Tests and scenario assertions re-execute; verdicts are never
  restored from cache.
- Start the measured runner instances only when their phases begin.
- Three consecutive rehearsals must complete within 5:30, preserving 30 s
  of contingency inside the six-minute slot.

## Presenter view

Simulator as the main visual; one restrained adjacent terminal. Show only:
robot/cube scene and episode type; observation age, policy decision, and
task-action dispatch count; runner identity and clean-output confirmation;
actual IB cache/remote/local work and elapsed time; candidate identity and
test/scenario result. Colors map to real states: failure, hold/rejection,
success. The rendering follows actual simulation state — never a separate
animation.
