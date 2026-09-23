# Recorded safety-gate replay

[简体中文](README.md)

`index.html` presents one question: **did the task complete, and did the freshness
contract pass?** These are separate results. A cube can lift while the gate violates
its policy.

Open the page directly in a browser; there is no build step, server, robot connection
or network requirement. Fonts and recorded data are bundled with the site. The page
defaults to Simplified Chinese; choose **EN** or use `?lang=en`.

## Follow the comparison

The replay opens with the seeded defect, then offers the repaired gate under the same
stale input, followed by the repaired gate with current input:

| Key / episode | Task completed? | Freshness contract passed? | Committed event log |
| --- | --- | --- | --- |
| **1 · Seeded defect · 600 ms** | **Yes** — 4 dispatches, cube at 0.9985 m | **Failed** — observations exceeded the 250 ms policy | [`ec2-runner-a/events-stale_600ms.jsonl`](../examples/ec2-runner-a/events-stale_600ms.jsonl) |
| **2 · Repaired · 600 ms** | **No** — arm held, 0 dispatches | **Passed** — stale input was refused | [`ec2-runner-b/events-center-f600.jsonl`](../examples/ec2-runner-b/events-center-f600.jsonl) |
| **3 · Repaired · 0 ms** | **Yes** — 4 dispatches, cube at 0.9978 m | **Passed** — observations were current | [`ec2-runner-b/events-center-f0.jsonl`](../examples/ec2-runner-b/events-center-f0.jsonl) |

The first episode ends with `success=true` at tick 65, even though it acts on
600 ms old observations. This is the reason for a protected assertion independent
of simulator task success. The repaired stale episode ends at tick 12; the fresh
episode ends at tick 54.

Below the replay, the table shows all 17 recorded episodes from the repaired build:
five cube placements × three observation ages, plus emergency-stop and timeout cases.
The evidence strip describes that recorded repaired build, including its verifier
result and executable digest; it does not claim a new simulator run occurred when
you opened this page.

## Presenting and inspecting

- **1 / 2 / 3** select the episodes in the order above.
- **Space** plays or pauses when focus is on the page. On a focused button, Space
  retains its normal button action.
- **Right arrow** pauses and shows the next recorded decision; one final step shows
  the episode result.
- **R** resets the current episode to its paused starting point.
- **Show result** immediately displays all decisions and both final verdicts.
- Switching browser tabs pauses playback. A reduced-motion preference starts each
  episode paused; Play and Next step remain available.
- Switching languages preserves the episode and playback position, and translates
  the current verdicts, controls and trace explanations.

Deep links select an episode, for example
[`index.html?play=seeded&lang=en`](index.html?play=seeded&lang=en).
The accepted values of `play` are `seeded`, `stale` and `fresh`; unknown values fall
back to the seeded episode. Selecting an episode updates the link without reloading.

## What is recorded, and what is schematic

Ticks, observation ages, gate decisions, dispatch counts, episode outcomes and final
cube heights are transcribed from run `ec2-e2e-20260923-160725`. The protocol panel is
a readable summary; the source link opens the original JSONL. The arm poses and
playback pacing are illustrative. They are **not recorded joint coordinates or a
MuJoCo video**; no video was recorded for this run.

To inspect the underlying messages from the repository root:

```sh
python3 - <<'PY'
import json
from pathlib import Path
path = Path('docs/examples/ec2-runner-a/events-stale_600ms.jsonl')
for line in path.read_text().splitlines():
    event = json.loads(line)
    message = json.loads(event['line'])
    if message.get('type') == 'proposal':
        age = (message['simulation_time_ns']
               - message['observation']['capture_time_ns']) // 10**6
        print(message['simulation_tick'],
              message['proposed_action']['segment'], f'{age} ms')
    elif event['dir'] == 'out':
        print('  ->', message.get('decision'), message.get('reason', ''))
    elif message.get('type') == 'episode_end':
        print(message)
PY
```

The 250 ms policy is a demonstration value, not a physical robot limit. Simulation
is not hardware validation. Build phases measured 21.4 s and 21.9 s; they demonstrate
that the Incredibuild build path ran, not a measured speedup.
