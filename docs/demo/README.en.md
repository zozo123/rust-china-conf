# Stage visual

[简体中文](README.md)

`index.html` is a single self-contained page that replays the safety gate's recorded
decisions. Open it directly in a browser — no server, no build step, and it works
offline (the IBM Plex webfonts degrade to a system stack if there is no network).

## What it shows

Three episodes, each replayed from the committed event logs in `docs/examples`:

| Button | Source | Point it makes |
| --- | --- | --- |
| Stale episode · 600 ms | `ec2-runner-b/events-center-f600.jsonl` | The patched gate refuses a 600 ms old observation and dispatches nothing. |
| Fresh episode · 0 ms | `ec2-runner-b/events-center-f0.jsonl` | With current information the same build permits all four segments and lifts the cube. |
| Seeded build · same stale input | `ec2-runner-a/events-stale_600ms.jsonl` | The seeded build permits all four segments on that same stale observation and reports success. |

The third one is the one worth pausing on: the episode "passes". The cube reaches
0.9985 m and the simulator reports `success=true`. Nothing in the run looks wrong,
which is the reason the protected assertion has to exist.

Below the replay, the matrix shows all 17 episodes from the patched build — five cube
placements against three observation ages, plus the emergency-stop and protocol-timeout
cases — and the evidence strip carries the verifier result and the digest of the
executable that produced these decisions.

## Driving it

Press `1`, `2` or `3` to switch episodes without a pointer. A slide can also link
straight into one with `index.html?play=seeded` (`stale`, `fresh`, `seeded`).

## Provenance

Every tick number, observation age, decision, dispatch count and cube height is
transcribed from run `ec2-e2e-20260923-160725`. To re-derive them:

```sh
python3 - <<'PY'
import json
for l in open('docs/examples/ec2-runner-a/events-stale_600ms.jsonl'):
    e = json.loads(l); m = json.loads(e['line'])
    if m.get('type') == 'proposal':
        age = (m['simulation_time_ns'] - m['observation']['capture_time_ns']) // 10**6
        print(m['simulation_tick'], m['proposed_action']['segment'], f'{age}ms')
    elif e['dir'] == 'out':
        print('   ->', m.get('decision'), m.get('reason', ''))
PY
```

The arm drawing is a schematic, not a MuJoCo frame capture — the run did not record
video. It is labelled as such on the page. Cube heights and all decision data are real.

One thing the page deliberately does not claim: the two build phases measured 21.4 s
and 21.9 s, which shows the accelerated path ran on both. That is not a speedup
measurement, and the page says so rather than implying a benchmark.
