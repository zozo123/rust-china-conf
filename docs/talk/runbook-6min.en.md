# Six-minute stage runbook

[简体中文](runbook-6min.md) · [Talk](talk-25min.en.md)

## Before the session

Commit the intended source revision. Runner scripts execute that committed
revision; `python3 scripts/robot-demo/check.py` checks edits before committing.

On the Linux initiator, select the real simulator and require IB:

```bash
export REQUIRE_IB=1
export ROBOT_DEMO_BACKEND=robosuite
export IB_ALLOW_CLEAR_USER_CACHE=1
export IB_HISTORY_URL='https://<coordinator>:8000/api/builds?coordinatorId=<id>&version=1.5.0'
export IB_CLIENT_API_KEY='<local secret>'
export ROBOT_DEMO_PYTHON="$PWD/demo/robot-sim/.venv/bin/python3"
scripts/robot-demo/preflight.sh
scripts/robot-demo/ec2-agentic-physical-ai.sh rehearsal-unique-id
```

This clears only the current user's local IB cache. Retain the raw Build History
and cache telemetry, Rust proof receipt, full 17-case evidence, executable and
verifier report. Confirm the simulator initializes and the result meets the
contract. Do three timed rehearsals under 5:30. A script's printed stage labels
are not elapsed timings.

Keep the local site, media and a complete labeled recording on the Mac.
The historical replay is `docs/demo/index.html?play=seeded&lang=en`.

## The visible sequence

| Time | Action | Narration |
| --- | --- | --- |
| 0:00–0:45 | Seeded lift; show task and freshness verdicts together | “The cube lifted. The freshness contract failed.” |
| 0:45–1:30 | Inspect the bounded patch; external agent attempt if configured | “The candidate cannot redefine acceptance.” |
| 1:30–2:00 | Export patch and base; show workspace A removed | “Keep the change and evidence. Remove the workspace.” |
| 2:00–3:00 | B builds from the recorded base with fresh outputs | “Build this candidate, then execute its checks again.” |
| 3:00–4:30 | Patched stale refusal, then fresh lift | “Zero stale dispatches; the fresh task still completes.” |
| 4:30–5:30 | Evidence receipt, scope and actual timings | “These results belong to this executable and this run.” |
| 5:30–6:00 | Contingency | Switch to the labeled recording if needed. |

If only two selected cases run live, label the completed 17-case matrix as
**rehearsal coverage**. Do not imply that it just ran.

## Driving the recorded replay

- **1:** seeded defect on stale input.
- **2:** repaired gate on stale input.
- **3:** repaired gate on fresh input.
- **Space:** play/pause when focus is on the page.
- **Right arrow:** next decision, then final result.
- **R:** reset to a paused start.
- **Show result:** jump to the final verdicts.

The arm poses are schematic; the event data comes from the recorded run.
For real simulator footage, use the landing page's separately labeled video.

## Running individual phases

Use unique IDs; existing evidence directories are intentionally rejected.

```bash
scripts/robot-demo/cold.sh stage-a
# cold.sh exports the evidence and removes A before returning.
# Use an external agent here if available; cap it at approximately 45 seconds.
# Otherwise explicitly select the reviewed fallback:
export ROBOT_DEMO_PATCH_FILE="$PWD/demo/fallback-patch.diff"
export ROBOT_DEMO_BASE_REVISION="$(cat evidence/stage-a/agent-context/base-revision.txt)"
scripts/robot-demo/warm.sh stage-b
scripts/robot-demo/validate.sh stage-b
```

The default script uses the reviewed patch, not an automated LLM call.
The allowlist checks the candidate before it is applied.
The full matrix runs in B; if that does not fit the stage budget, show the
completed rehearsal and label it, rather than claiming a smaller live run is
the full suite.

## What to say accurately

- Today's runner is a temporary worktree on an EC2-backed Linux host.
  Removing it does not destroy the host. Islo integration is planned.
- New run results and historical results have different IDs and may have
  different check counts. The old 10/10 and 88/88 describe the archived revision.
- Historical build observations were 21.426 s and 21.948 s. B was 522 ms slower.
  They remain historical and uncontrolled.
- The new experiment compares native, IB cold, and IB parent-warmed builds of
  one candidate with at least five samples per mode. Claim acceleration or
  reuse only if the Rust verifier passes remote-task, remote-core-time, and
  cache-hit checks and the generated receipt supports the claim.
- This is software-in-the-loop with segment-level authorization.
  Physical robot validation and continuous control-step supervision are outside scope.
