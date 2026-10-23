# Website media

The landing page hero uses `robot-stale_600ms.mp4` and `robot-stale_600ms-poster.png`: the seeded build lifting the cube on 600 ms-old observations (details below). `robot-lift.mp4`, `robot-lift.gif` and `robot-lift-poster.png` are recorded robosuite / MuJoCo fresh-lift frames; the story page shows `robot-lift.gif` beside the stale clip. None of this is a live robot feed. Playback is user-controlled.

Regenerate simulator footage in the pinned Linux environment with `scripts/robot-demo/record-gif.sh`. Website media does not replace digest-bound run evidence in `docs/examples/`.

The `validation-loop*` videos, GIFs and posters are legacy walkthrough assets. Their historical cache/runner terminology is not proof of cache reuse, helper activity or VM destruction. They are retained for old links; use the current landing page and interactive replay for the talk.

`robot-stale_600ms.{gif,mp4,png}` are the STALE episode — the one the talk opens on.
Recorded from run `asset-stale-verify-1790261498`, whose `scenario-results.json` reads
`scenario: stale_600ms · outcome: cube_lifted · success: true · rejections: []`
on robosuite 1.5.2 / mujoco 3.9.0. The task succeeds; the freshness contract does not.
Regenerate with `scripts/robot-demo/record-scenario-gif.sh` and `SCENARIO=stale_600ms`.
Unlike `robot-lift.*`, this footage shows the failure the talk is about.
