# Six-minute stage runbook

[简体中文](runbook-6min.md) · [25-minute talk](talk-25min.en.md) · [Slides](slides.en.md)

Allocations are rehearsal budgets, not claimed runtimes. One complete
six-minute recording stays available locally; if network or rendering fails,
play the recording — visibly labeled — rather than debugging live.

Automation: `scripts/robot-demo/rehearse.sh <run-id>` performs the whole arc
(`ROBOT_DEMO_BACKEND=robosuite` for the real SIL path). On stage, run the
phases individually so narration stays in charge of pacing.

Fallback if the grid is unreachable: `docs/demo/index.html` replays the
recorded run with no network, no server and no build step. It is labeled as a
replay on its face, so showing it is not a misrepresentation — but say
"this is the recorded run" out loud anyway.

| Time | Stage action | Command / artifact | Number to say |
|---|---|---|---|
| 0:00–0:30 | Show Panda/cube; introduce the stale observation | simulator view; `demo/robot-sim/config/scenarios/stale_600ms.json` | "600 milliseconds" |
| 0:30–1:15 | Runner A cold build + the failing episode | `scripts/robot-demo/cold.sh <run>-runner-a` | 21426 ms, `ib: true`; 4 dispatches at ticks 12/22/32/48 |
| 1:15–2:05 | Agent produces bounded patch from failure context (cap ≈45 s) | work order: `evidence/<run>-runner-a/agent-context/work-order.md` | — |
| 2:05–2:30 | Show the diff, export context, **destroy A** | `demo/fallback-patch.diff` | 49 lines, 4 of intent |
| 2:30–3:15 | Fresh runner B builds through IB, runs checks | `scripts/robot-demo/warm.sh <run>-runner-b` | 21948 ms, `ib: true`; 10/10 tests |
| 3:15–4:00 | Patched executable refuses the stale episode; show **zero** dispatches | matrix row `center-f600` → `rejected_stale` | refused at tick 12, 0 dispatches |
| 4:00–4:40 | Fresh episode lifts the cube | matrix rows `*-f0` → `cube_lifted` | 4 permits, cube 0.9978 m |
| 4:40–5:20 | Evidence pack + build comparison | `evidence/<run>-runner-b/`, `build-metrics.jsonl` | 17 episodes; 88/88; digest f358e898 |
| 5:20–5:40 | Simulation scope; downstream hardware gate | scope statement in every manifest | — |
| 5:40–6:00 | Buffer | — | — |

## The one beat that carries the talk

At 0:30–1:15, do not rush past the seeded failure. The episode reports
`success=true` and lifts the cube to 0.9985 m. Say that the run is *green*
and let the room sit with it for a beat before revealing that all four
dispatches acted on a 600 ms-old observation. The demo's argument is not
"the build broke" — it is "the build passed and was wrong."

If you are using the stage visual, `docs/demo/index.html?play=seeded` opens
straight into this episode. Keys `1`/`2`/`3` switch between the stale, fresh
and seeded episodes without a pointer.

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
- **Never state a speedup.** The two build phases are 21426 ms and 21948 ms;
  the warm one was slower. If asked live, say: "I measured that the
  distributed path ran on both phases. I have not measured a speedup, and
  the benchmark to do it properly is specified in the repo."

## If someone asks about the machines

Answer plainly: a real Incredibuild 4.31.0 grid — one coordinator, one
initiator, two helpers, on our own Linux hosts. The islo sandbox path is in
the 25-minute script as the honest version: five resource limits, then the
account ran out of credit. Do not present the verified run as having
happened on islo; the run IDs, manifests and evidence README in the repo all
record EC2, and the contradiction is one `ls` away.

The architectural point survives either answer, which is exactly the point:
a fresh worktree, a fresh `CARGO_TARGET_DIR`, a directory of evidence files
and a digest-bound verdict moved between two unrelated infrastructures
without a line changing.

## Presenter view

Simulator as the main visual; one restrained adjacent terminal. Show only:
robot/cube scene and episode type; observation age, policy decision, and
task-action dispatch count; runner identity and clean-output confirmation;
actual IB cache/remote/local work and elapsed time; candidate identity and
test/scenario result. Colors map to real states: failure, hold/rejection,
success. The rendering follows actual simulation state — never a separate
animation.
