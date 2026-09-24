# The Build Cache verdict — both rows

*Measured 2026-09-24 on AL6555 initiator (4 cores), rustc 1.92.0, 45-package workspace,
profile `rustc type="local_only"` with `<ib_cache enabled="true"/>`. Distribution confirmed
absent every time: Build History reports 0 helpers and `max_initiator_cores=0`.*

## The result depends entirely on one thing: is your target directory warm?

| scenario | plain cargo | IB cache-only | winner |
|---|---|---|---|
| **target directory kept** (a developer rebuilding) | **939 ms** | 4,048 ms | **cargo, 4.3x** |
| **target directory empty** (fresh workspace / fork / CI) | 11,515 ms | **3,706 ms** | **cache, 3.13x** |

Cache participation is not inferred. `--build-cache-report-all-miss` wrote
`/etc/incredibuild/log/2026-Sep-24/local-95/ib_hm.log`:

```
HIT lines : 52
MISS lines: 0
```

and the coordinator's build report shows the cached build executed **9 tasks instead of 58** —
the other 49 compilations never invoked `rustc` at all.

## What this means, stated plainly

**The Build Cache does not make your build faster. It makes throwing your workspace away
cheap.**

On a machine with an intact `target/`, cargo's own incremental state already holds the answer,
and the cache only adds interception overhead — 4x worse. Start from nothing, and cargo must
recompile all 52 units while the cache serves them — 3.13x better.

That is precisely the claim the project's third pillar makes: *disposable workspaces, reusable
compilation.* The workspace is disposable; the compilation is not. An agentic loop that forks
an isolated workspace per candidate pays the empty-target cost on every candidate, which is
the row where the cache wins.

## Correction history, because it matters more than the number

1. A first run reported 1.76x and 3.13x. `ib-benchmark.sh:449` had `native_sample()` call
   `disposable_workspace()`, wiping `CARGO_TARGET_DIR` before every native sample, so the
   baseline was cargo-from-nothing.
2. That was retracted as "the cache is slower", measured against a warm-target baseline.
   **That retraction was also incomplete** — it answered the developer question and presented
   it as the whole verdict.
3. A verification run then used a profile with `type="allow_remote"` while calling the result
   "cache-only". It happened not to distribute (3 tasks, 0 helpers), so the number survived,
   but the setup did not support the claim.

Both rows above are the honest answer. Neither alone is.

## Limits

* One workspace, 45 packages, ~11.5 s cold. Whether the fixed per-invocation overhead
  amortises at larger scale is unmeasured here; a 576-package run was in flight.
* `BuildCache.ServiceURL` is empty on this grid, so cross-machine cache sharing is not
  demonstrated — only same-machine reuse.
* No distribution result is claimed: every build above ran with 0 helpers, deliberately.
