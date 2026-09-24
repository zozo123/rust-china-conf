# DEMO RESULTS — what we actually ran on 2026-09-24, and what it proves

**Everything in this document was produced by a command run on this Mac today.** No number is
reused from an earlier evidence bundle. The AL6553 grid was unreachable for the whole run (VPN
portal not routable from this LAN; `10.133/16` has no route). **No host was ssh'd to. No
Incredibuild binary exists on this machine. No acceleration was measured and none is claimed.**

Repo: `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf`
Branch `fix/proof-layer-and-public-claims`, HEAD `2883e19a166cedcb4075c37fb7a0cfb148b0d540`
(PR #2). Tree clean before, during handover, and after: `git status --porcelain` empty,
`git worktree list` shows only the main checkout. Nothing committed, nothing pushed.

Machine: macOS 26.6.2 (25G83), arm64, 10 logical cores, cargo/rustc 1.92.0,
`demo/robot-sim/.venv` = Python 3.12.12 with robosuite 1.5.2 / MuJoCo 3.9.0 / numpy 1.26.4.

---

## 1. TL;DR

Three things ran end to end, all local, all on the **real robosuite SIL backend** (not mock):

1. **The full robotics rehearsal arc** — preflight → runner A (seeded bug) → bounded patch →
   runner B (repaired) → protected verification. Run id `local-e2e-20260924T125134`.
2. **The first controlled build experiment this repo has ever produced** — 20 timed builds,
   4 configurations × 5 samples, through the real `cache-clear.sh → build-sample →
   build-receipt → build-proof` pipeline. Run id `build-exp-20260924T100157`.
3. **An adversarial pass over both**, which re-ran every load-bearing command from disk rather
   than trusting the run logs, and which found one thing that matters more than either.

**The one honest headline number: the whole rehearsal arc took 76.58 s — 21.3% of the 360 s
stage budget.** Measured as the span of a timestamped wrapper log around `rehearse.sh`
(`1790243508.602` → `1790243585.182`). Everything the talk needs to show live fits, with room.

**And the one finding that outranks it:** `build-proof` **accepts a hand-forged receipt.** We
built one — 5 real native samples plus 10 hand-written Incredibuild samples with invented
counters, a cache-clear transcript pointing at `/tmp/does-not-exist.txt`, and a
`transcript_sha256` of 64 zeros — and the validator printed `BUILD RECEIPT CONSISTENT` with a
fabricated `11.948x` acceleration ratio and exited 0. Reproduced today, twice, by two
independent passes. See §5.2. This is not a defect that weakens the talk; it is the talk's
thesis landing on the talk's own tooling, and it is the most valuable thing this run produced.

---

## 2. The robotics demo — run `local-e2e-20260924T125134`

Backend: **robosuite 1.5.2 / mujoco 3.9.0** on every one of the 18 episodes. Confirmed three
ways: the manifests record `"backend": "robosuite"`; all 17 rows of `scenario-results.json`
carry `"backend": "robosuite 1.5.2 / mujoco 3.9.0"` (distinct from the mock's self-labelling
`"mock (kinematic stand-in; NOT robosuite SIL)"`); and `bridge-stderr.log` contains 17
`backend=robosuite` lines, zero occurrences of "mock", and genuine MuJoCo startup noise
(`macros.py:57`, `composite_controller_factory.py:121` loading `default_panda.json`).

### 2.1 The safety invariant, in the words the source and the tests actually pin

Rule 3 of `decide` in `robot-safety-gate`:

> **Freshness at the final dispatch decision: observations older than the configured threshold
> are rejected as stale. Boundary: exactly `max_observation_age_ms` permits; one millisecond
> beyond rejects.**

Formally, as enforced by `rust/crates/robot-safety-gate/tests/contract.rs`:

```
age_ms <= policy.max_observation_age_ms  ->  Permit
age_ms >  policy.max_observation_age_ms  ->  Reject(StalePerception { age_ms })
DEFAULT_MAX_OBSERVATION_AGE_MS = 250
```

The work order states the same thing as *"At dispatch, observations older than 250 ms must
return `StalePerception`."*

Two adjacent invariants were **exercised, not merely asserted**:
- **Emergency stop outranks staleness.** `estop_descend` → `emergency_stop`;
  `stop_with_stale_data_rejects_emergency_stop` passes on the repaired gate.
- **Hold steps never pass through the gate.** They are explicitly unguarded, and the protected
  verifier counts them separately — "13 labeled hold(s)" on each `f600` scenario, i.e. the arm
  holds position rather than dispatching. Confirmed in `events-left-f600.jsonl`: 12
  staleness-injection hold ticks advancing `simulation_time_ns` 50e6 → 600e6, a proposal whose
  observation `capture_time_ns` is 0 at `simulation_time_ns` 600000000 (age exactly 600 ms),
  the gate line `{"decision":"reject","reason":"stale_perception","age_ms":600}`, then a 13th
  hold labelled `rejection:stale_perception`, then `episode_end success=false
  reason=rejected_stale`.

### 2.2 Half (a) — "The robot succeeded. The contract failed."

Runner A built the committed **seeded** revision `2883e19` in a detached worktree and ran
`stale_600ms` on robosuite:

```
outcome=cube_lifted  success=true  task_dispatches=4  decisions=4  ticks=69  wall=2066 ms
```

A 600 ms stale observation was dispatched and the cube was lifted. On the same revision:

```
$ cargo test -p robot-safety-gate --test contract
FAILED. 5 passed; 3 failed
  boundary_251ms_rejects
  configured_threshold_is_respected
  stale_ages_are_rejected
```

with real panic text (`age 251 ms must be rejected as stale`, `left: Permit`). **Task green,
contract red, on the same binary.** That is the hook, and it is real.

The protected verifier, run against runner A's evidence **as a single-scenario diagnostic**
(`--scenario stale_600ms`), returns exit 1 and:

```
PROTECTED VERDICT: FAIL (1 scenario(s) failed verification)
  FAIL  stale_600ms: outcome 'cube_lifted' != expected 'rejected_stale'
```

> **Wording discipline.** This is a *single-scenario diagnostic*, not matrix acceptance.
> `verify_run.py`'s own docstring (line 5) says so. Do not phrase it as "the protected matrix
> failed on runner A" — the matrix was never run against runner A.

### 2.3 Half (b) — the bounded patch repairs it

- `check-patch.py` passed the candidate allowlist: `candidate allowlist passed:
  rust/crates/robot-safety-gate/src/lib.rs` (exit 0). `runner-b/candidate.patch` is
  byte-identical to `demo/fallback-patch.diff`.
- `git apply` landed it on the exact base revision recorded in runner A's `base-revision.txt`.
- Runner B rebuilt from scratch (fresh `CARGO_TARGET_DIR`). Contract tests **8 passed / 0
  failed**; `robot-safety-gate` unit 2/2; `swf-app` suites 9/9 and 17/17 — **36 tests total on
  the repaired revision.**
- The 17-scenario matrix ran on robosuite with **0 infrastructure failures**.
- Protected verification: `PROTECTED VERDICT: PASS (17 scenarios; complete coverage matrix)`,
  exit 0, re-run independently today against a manifest bound to sha256
  `72e2694ebc0e8b2cb37e3993bb770e51f7d15af0ccb4755c069b411249881a45`.

> **Correction to the brief.** The brief says the repaired workspace is "27/27". It is **36**:
> 2 unit + 8 contract + 9 + 17. Anyone who goes looking for 27 will not find it.

> **Wording discipline on the sha256.** `verify_run.py` contains **no subprocess call**. It
> re-derives the verdict from the recorded event traces and hashes the binary for *identity*
> (lines 193-202); its docstring (line 8) states the digest "is not independent proof that
> untrusted evidence was produced by that executable." Correct phrasing on stage: *"the
> verifier re-derives the verdict from the recorded trace, against a manifest bound to a
> sha256-identified executable."* Not "run against the executable."
>
> The binding itself is real: `shasum -a 256` on `runner-b/artifact/swf-cli` recomputes to
> `72e2694e…881a45`, matching the manifest field **and** the `.sha256` sidecar. Runner A's
> binary is a **different** digest, `6003f26d41bf78173ef2f680569ef559deffbf8febeeaeedfc4b39d6df63fe2e`
> — two builds, two digests, not one binary relabelled.

### 2.4 The 17-scenario matrix

| Outcome | n | Scenarios |
|---|---|---|
| `cube_lifted` | 10 | 5 placements × freshness f0 and f50 |
| `rejected_stale` | 5 | 5 placements × f600, each `reject/stale_perception(age=600ms)` |
| `emergency_stop` | 1 | `estop_descend` |
| `timeout` | 1 | `protocol_timeout` — intentional, expected |

**Say it precisely.** "17 PASS / 0 FAIL" means *17 of 17 matched their required outcome*, which
is verifier-expectation pass, **not** task success. Only 10 rows carry `"success": true`; the
five `f600` rows, `estop_descend` and `protocol_timeout` all record `"success": false` —
correctly, because refusal **is** the expected outcome. On a stage whose thesis is that green
checkmarks get over-read, "17 PASS" beside "the robot succeeded" is exactly the ambiguity the
talk exists to attack. Use:

> *"17 of 17 matched their required outcome: 10 lifted the cube, 5 refused on staleness, 1
> e-stopped, 1 timed out as designed."*

Wall times (recomputed from `scenario-results.json` today): sum of the 17 reported episode
walls **35 560 ms**; longest single scenario `protocol_timeout` **9 519 ms**; the 15
placement × freshness scenarios span **1 341–2 434 ms**, mean **1 642.8 ms**.

---

## 3. The build experiment — run `build-exp-20260924T100157`

> **This is a NATIVE-ONLY measurement on a MacBook and it establishes NOTHING about
> Incredibuild.** `command -v ib_console` → not found. `REQUIRE_IB` was never set. Every one of
> the 20 timed builds is a plain `cargo build --workspace --locked`. Preflight printed
> `warn ib_console not found — native cargo baseline (labeled; no acceleration claimed)`.
> Every `build-metrics` record in both runners reads `"build_provider": "native", "ib": false,
> "cache_reuse_verified": false`. All 20 receipt samples are `mode: "native"` with every
> IB-only field (`remote_tasks`, `local_tasks`, `remote_core_time_s`, `cache_hits`,
> `cache_misses`) **null** — verified programmatically, zero non-null.

### 3.1 The table

Design: two detached worktrees off `2883e19` — `candidate` (patch applied permanently) and
`warmseed` (parent revision, patch applied/reverted per warm sample). Configurations rotated
per repetition. `cargo fetch --locked` run **outside** every timed interval; no timed build log
contains a download.

| Config | n | Median | Range | Spread |
|---|---|---|---|---|
| cold-j1 (cache cleared, `cargo -j1`) | 5 | **22 861 ms** | 22 629 – 23 236 ms | 607 ms |
| cold-j10 (cache cleared, `cargo -j10`) | 5 | **6 976 ms** | 6 777 – 7 766 ms | 989 ms |
| warm-j10 (parent-seeded, patch applied) | 5 | **942 ms** | 915 – 984 ms | 69 ms |
| warm-j1 (parent-seeded, patch applied) | 5 | **1 016 ms** | 1 014 – 1 099 ms | 85 ms |

Derived ratios — **medians only, and quoted here with the envelope the raw min/max actually
support**, because four significant figures at n=5 with no interval is false precision:

| Ratio | Median-to-median | Envelope from sample min/max |
|---|---|---|
| Parallelism when cold (j1/j10) | 3.277× | 2.91× – 3.43× |
| Parallelism when warm (j1/j10) | 1.079× | 1.03× – 1.20× |
| Parent-seeded cargo cache reuse, j10 | 7.406× | 6.89× – 8.49× |
| Parent-seeded cargo cache reuse, j1 *(single-threaded control, not a result)* | 22.501× | 20.59× – 22.92× |

**Read `22.501×` as a control, or drop it.** It pairs the slowest configuration anyone could
run (cold, single-threaded) against warm single-threaded on a 10-core machine. Nobody builds at
`-j1`. Printed beside `7.406×` it makes the reuse effect look scale-independent when the data
show the opposite.

**What it means for the talk:** 10 cores buy **3.3×** on a cold full-workspace build — far
short of 10×, because the crate graph serializes. A cold `-j10` rebuild of the whole workspace
is **~7 s**; the warm rebuild after the bounded patch lands is **~0.9 s**. The 6-minute stage
budget is safe either way. All of this is **native cargo target-dir reuse**. No Incredibuild is
involved anywhere in it.

**Caveat on "independent samples":** the *warm* half is repeated measures, not independent. One
`warmseed` worktree is reused with the patch applied and reverted per sample, and every warm
parent seed ran at `parent_seed_jobs=10` regardless of whether the timed build was `-j1` or
`-j10`. Both facts are disclosed in `method.txt` and visible in `raw/*.parent-seed.txt`. The
10 untimed parent-seed builds (6 749 – 7 951 ms) are excluded from every median above.

### 3.2 The receipt, and why `build-proof` rejected it

**The repo now has its first `receipt.json` ever.** `git log --oneline --all -- '*receipt.json'`
is empty and `git ls-files | grep -c receipt.json` is 0, so "none has ever existed" is true in
the git sense. Today's:

- **10 142 bytes**, sha256 `a42d40f3ea2cbf5a03e6fe3485dfc994d5529fe9d95b4c98073b2988e10ff5d2`,
  `schema_version` 2, 20 samples.
- Byte-identical at all three saved locations (verified by `shasum -a 256`).

`build-proof` **rejects it**, exit 1:

```
$ swf-cli robot-demo build-proof --receipt .../receipt.json --min-samples 5
Error: missing benchmark mode ib-cold
```

Same message on all four per-config receipts (4/4). This is structural, not a workaround
candidate: `validate_build_proof` iterates `["native", "ib-cold", "ib-parent-warm"]` and
`with_context`s a missing mode into that bail. **A native-only receipt cannot pass, by design.**

Negative controls, all reproduced today:

| Control | Result |
|---|---|
| 4-sample receipt | `Error: native has 4 sample(s), require at least 5`, exit 1 |
| Doctored IB telemetry on a native sample | `Error: native sample 1 contains IB-only telemetry`, exit 1 |
| `build-sample --cache-clear` on a native sample | `Error: native samples do not use the Incredibuild cache and must not claim a clear`, exit 1 (`rust/crates/swf-cli/src/main.rs:547`) |

### 3.3 Three things about that receipt that must be said out loud

**(a) It cannot distinguish the four configurations.** All 20 samples carry `mode: "native"`
and an identical `source_revision`. The cold/warm and j1/j10 distinction survives **only as a
substring of the free-text `build_caption`**. `mode_stats` (`main.rs:619-633`) takes one median
per mode and `print_build_proof` (`main.rs:977-986`) uses `stats["native"].median_ms` as the
denominator for every IB ratio. **That median is 3 938.0 ms — the midpoint of a bimodal
distribution, matching no build anyone ran.** The four per-config receipts (5 homogeneous
samples each) are the honest containers. The 20-sample primary must **never** become the
baseline a future grid receipt is divided into.

**(b) The 20 cache-clear transcripts are real but are not bound to the receipt.** All 20 exist
under `build-proof/raw/`, each begins `# swf-cache-clear v1`, each records `exit_code=0`, and
each `completed_at_ms` precedes its sample's `started_at_ms` (13-20 ms for the 10 cold samples;
6 899-8 000 ms for the 10 warm ones, the gap being the untimed parent-seed build, exactly as
`method.txt` documents). But native samples carry `cache_clears: []` by construction, so **no
digest or path for any of them appears anywhere in `receipt.json`.** They are retained raw
evidence. They are not part of the receipt.

**(c) The transcripts record less than they appear to.** `cache-clear.sh` shifts `TOOL` off the
argument list before writing `argv=$*`, so every transcript reads
`argv=-rf /var/folders/.../target-cold` with **no tool name** and an empty body — and `rm -rf`
on a nonexistent path also exits 0. They record that *some unnamed tool* exited 0 with those
arguments. **On the grid this same transcript is the only evidence `build-proof` accepts for
the `ib-cold` and `ib-parent-warm` cache clears.** Record the tool name before the grid run.

> Also precise: **the receipt has no `build_provider` field.** Samples carry `mode: "native"`.
> `build_provider: native` appears in `method.txt` and in the e2e `build-metrics.jsonl`, not in
> the receipt. And the "scope derived as `unknown`" statement is a **reading of the rule at
> `main.rs:477`** (any argv not starting with `user`/`shared`/`service`/`all`/`global` →
> `Unknown`), not an observed output — `swf-cli` never parsed those transcripts in this run,
> because native samples refuse `--cache-clear`.

---

## 4. What this unlocks for the talk

Beats that were previously "implemented, not demonstrated" now have a run from **today** behind
them, on this Mac, offline.

| Beat | Now backed by | Evidence path |
|---|---|---|
| The hook: task succeeds, contract fails | `stale_600ms` on robosuite, `cube_lifted`/`success=true` + 3 red contract tests on the same revision | `rust-china-conf/evidence/local-e2e-20260924T125134-runner-a` |
| Bounded candidate patch, allowlist-enforced | `check-patch.py` pass, patch byte-identical to `demo/fallback-patch.diff` | `…-runner-b/candidate.patch` |
| Repaired gate, full coverage matrix, robosuite | 17/17 matched outcome, 0 infrastructure failures | `rust-china-conf/evidence/local-e2e-20260924T125134-runner-b` |
| Protected verdict, both directions | `PASS (17 scenarios…)` exit 0 and `FAIL (1 scenario(s)…)` exit 1, both re-run today | same two bundles |
| sha256 artifact binding | two distinct digests, each matching manifest + sidecar + recompute | `…-runner-{a,b}/artifact/swf-cli.sha256` |
| The proof pipeline actually runs | `cache-clear.sh → build-sample → build-receipt → build-proof`, 20 samples, first receipt ever | `outputs/evidence-live/build-proof/` |
| The gate refuses an incomplete receipt | `Error: missing benchmark mode ib-cold`, exit 1, plus 3 negative controls | `build-proof/negative-controls.txt` |
| **The gate accepts a fabricated one** | forged receipt → `BUILD RECEIPT CONSISTENT`, `11.948x`, exit 0 | §5.2 below |
| macOS-offline fallback for the whole robotics arc | the entire arc ran with no grid, no VPN, no `ib_console` | all of the above |

**Full evidence paths**

In-repo (not committed):
- `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf/evidence/local-e2e-20260924T125134-runner-a`
- `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf/evidence/local-e2e-20260924T125134-runner-b`
- `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf/evidence/build-exp-20260924T100157/build-proof/`
- `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf/evidence/check-20260924T095330888335Z` (`check.py` gate, **mock backend by design**)

Copied out, byte-identical (`diff -r` clean):
- `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/outputs/evidence-live/robot/`
- `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/outputs/evidence-live/build-proof/`
- `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/outputs/evidence-live/receipt.json`
- `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/outputs/evidence-live/robot/local-e2e-20260924T125134-rehearse.log`

### 4.1 Stage environment, established today

```
ROBOT_DEMO_BACKEND=robosuite
ROBOT_DEMO_PYTHON=<absolute path to demo/robot-sim/.venv/bin/python3>
MUJOCO_GL       unset          # NOT osmesa, NOT egl — see below
REQUIRE_IB      unset
```

**`MUJOCO_GL`: leave it unset on macOS.** Tested five values today. `unset`, `glfw` and `cgl`
all import and render offscreen identically. **`osmesa` and `egl` both hard-fail at
`import mujoco`** with `RuntimeError: invalid value for environment variable MUJOCO_GL` —
before any simulation starts. Those are precisely the two values a Linux runbook would carry
over. Never copy them.

**`ROBOT_DEMO_PYTHON` is a landmine and the brief had it backwards.** `runner-common.sh:11` and
`preflight.sh:17` are both `PYTHON="${ROBOT_DEMO_PYTHON:-python3}"`. Only `record-gif.sh:7`
defaults to the venv. Bare `python3` on this Mac is Homebrew CPython **3.14.7**, which has no
robosuite (`ModuleNotFoundError`, verified). An operator running `cold.sh`/`warm.sh` with
`ROBOT_DEMO_BACKEND=robosuite` and no `ROBOT_DEMO_PYTHON` export would hand `bridge.py` to
3.14.7 and fail mid-episode, **after the cache had been cleared.** The tell is a preflight that
prints `ok python: Python 3.14.7` and `ok robosuite 1.5.2 / mujoco 3.9.0` simultaneously —
because the venv probe is hardcoded to the venv path while the `$PYTHON` probe is not.

**Precedence, on this branch, is now correct and the brief's description of it is stale.**
`scripts/robot-demo/env-local.sh` is genuinely **fills-unset-only**: it sources `.env.local` in
a child shell, filters the result, exports only names that are unset, and prints
`note keeping caller-provided $NAME` for anything it declined to apply. It is sourced by
`preflight.sh`, `runner-common.sh`, `rehearse.sh` and `ib-benchmark.sh` — four stage scripts.
The old `set -a` after-the-exports behaviour is gone.

**But the fix is machine-local.** `.env.local` is gitignored (`.gitignore:10`). A fresh clone,
another operator account, or the grid host still hits the bare-`python3` default. "Durable"
means durable on this Mac only.

**Pins.** `requirements-linux.txt` pins robosuite 1.5.2 / MuJoCo 3.9.0 / numpy 1.26.4 and the
venv has precisely those; all 26 shared pins match byte-for-byte. Two pins are absent and both
are genuinely Linux-only with no macOS wheel: `evdev==2.0.0` and `python-xlib==0.33`, pynput's
Linux backend. On macOS pynput 1.8.2 pulls 5 pyobjc packages (all 12.2.2) instead. **Correct
platform substitution, not a downgrade** — no pin was moved off its recorded version. The
macOS environment is pin-identical to the archived Linux SIL run for every package that governs
simulation. Nothing was installed today: the venv was created 2026-09-23 14:06 and
`find .venv -newermt 2026-09-24 -type f ! -name '*.pyc'` is empty. Dependency acquisition was a
no-op, so nothing timed later is contaminated by it.

---

## 5. What is still missing

### 5.1 The Incredibuild receipt — still zero, and not obtainable here

No receipt this machine can produce will ever pass `build-proof`, and that is correct
behaviour. What has to happen:

1. **Grid reachable.** VPN route to `10.133/16` restored; the four AL6553 hosts reachable.
2. **`ib-benchmark.sh`'s hard requirements satisfied** — it refuses on this Mac by design
   (`scripts/robot-demo/ib-benchmark.sh:49-63`): `IB_ALLOW_CLEAR_USER_CACHE=1`,
   `IB_HISTORY_URL`, `IB_CLIENT_API_KEY`, `command -v ib_console`, and executable
   `/opt/incredibuild/management/{build_avoid_cache.sh,show_build_cache_statistics.sh}`. **None
   exist here.** This is the correct refusal, not a workaround candidate.
3. **Coordinator licensing working** — helper cores licensed, `remote_tasks > 0`.
4. **≥5 samples in each of `native`, `ib-cold`, `ib-parent-warm`**, with distinct repetitions,
   per-sample cache-clear transcripts, and Build History records with unique captions.
5. **Fix `cache-clear.sh` to record the tool name first** (§3.3c), or the transcripts that
   gate the IB modes on the grid will be as uninformative as today's.

Until all five hold, the talk stays on **"implemented, not measured."** That instruction is
still right.

### 5.2 The proof layer's own gap — found today, and it is the story

`build-proof` **reads only the receipt's own fields.** It never opens the transcript file, never
contacts Build History, never runs the cache-statistics tool. `check_clear_usable`
(`main.rs:695-700`) validates `transcript_sha256` for **shape only** — 64 chars in `[0-9a-f]` —
and never recomputes it against `transcript_path`, which is never opened. The digest *is*
genuinely computed from the real file, but only at **`build-sample`** time (`load_cache_clear`,
`main.rs:493`), and a receipt is a hand-editable JSON file afterwards.

Demonstrated, reproduced twice today:

```
$ swf-cli robot-demo build-proof --receipt forged-receipt.json --min-samples 5
BUILD RECEIPT CONSISTENT  run=build-exp-20260924T100157
native          median=  7766.0ms range=   942.. 22861ms
ib-cold         median=  2114.0ms range=  2100..  2128ms
ib-parent-warm  median=   650.0ms range=   640..   660ms
ib-cold         measured ratio=3.674x vs native; saved=5652ms
ib-parent-warm  measured ratio=11.948x vs native; saved=7116ms
cache scope observed in 10 clear transcript(s): local-user; 5 parent-seed build(s) attributed
CHECKED FROM RECORDS: ... one Build History record per caption reporting success; ...
  cold-cache hits==0 and warm-cache hits>0 from the cache-statistics tool; every IB build
  preceded by a transcribed local-user cache clear that exited 0 ...
NOT CHECKED: this is a consistency check over a receipt, not a proof. It cannot detect a
  fabricated receipt, and it does not observe the cache itself. ...
exit=0
```

The forged receipt is 5 real native samples plus 10 hand-written IB samples with invented
`remote_tasks=412` / `remote_core_time_s=880.5` / cache counters, and `cache_clears` whose
`transcript_path` is `/tmp/does-not-exist.txt` and whose `transcript_sha256` is **64 zeros**.

**Two consequences, and they cut in opposite directions.**

1. **Correct the run report.** What rejected the real receipt was a **missing mode**, not fraud
   detection. The claim "passing it would have required inventing Build History records and
   cache-statistics counters" is **wrong**: it required inventing nothing but JSON fields, and
   took about a minute.
2. **Fix the tool before the stage.** The `NOT CHECKED` paragraph is accurate and saves the
   tool's honesty. But the adjacent `CHECKED FROM RECORDS` line claims it verified "one Build
   History record per caption reporting success" and "cold-cache hits==0 and warm-cache hits>0
   from the cache-statistics tool" — and on the forged receipt **that entire line printed
   verbatim over wholly fabricated counters.** The two lines contradict each other, and
   `CHECKED FROM RECORDS` is the one a reader quotes. **This is the highest-value repo fix
   before the talk.** Either recompute the transcript digests and re-read the retained Build
   History responses at proof time, or reword the line to say what it actually checks:
   *"CHECKED FROM THIS RECEIPT'S OWN FIELDS."*

This finding is not a setback. A talk arguing that unverified claims get caught, which then
catches its own verifier over-claiming — for the second time, after the schema-v1 tautology —
is a stronger beat than the one it replaces. **The honest line: "we fixed this layer once,
found it was still over-claiming, and here is the forged receipt that proves it."**

### 5.3 Three numbers that rest on the runner's word alone

Not contradictions — limits. Everything else in this document was re-derived from disk.

- `check.py` gate "exit 0, 36 s wall": the retained `check-20260924T095330888335Z` directory
  has no verdict or timing file. It does confirm `backend: "mock"` and a clean `git_status`.
- The two robosuite bridge hello-latency probes (2.06 s / 1.23 s against an 8 000 ms recv
  timeout) were point measurements, not retained as artifacts. The bridge demonstrably works,
  so nothing is in doubt, but those two numbers are not on disk.
- `downloads_outside_timing=true` rests on the scratchpad script, not on an independent artifact.

### 5.4 Documents that now contradict the branch

- `outputs/WHAT-AND-HOW.md:7` and `:126` say "no controlled benchmark receipt has ever existed"
  and "No receipt has ever existed." One now exists. It is **rejected** by `build-proof`, so
  *"no **validated** receipt has ever existed"* remains true — but the flat sentence is false
  as written, and the file sits in the same directory as the receipt.
- `PREFLIGHT-AND-RUNBOOK.md` gate **P7** declared any `ROBOT_DEMO_PYTHON` line in `.env.local`
  a FAIL. On this branch that would instruct the operator to delete the line that makes the
  demo work. **Corrected in this pass.**
- `PREFLIGHT-AND-RUNBOOK.md` and `FINAL-TALK-E2E.md` described the old `set -a`-wins precedence.
  **Corrected in this pass.**
- The Beat 7 fallback pointed at yesterday's `evidence/sil-final-runner-b`. **Repointed to
  today's bundle in this pass**, with the old one retained as a second fallback.

---

## 6. Stage timing reality

Budget: **360 s**. Full arc measured: **76.58 s** = **21.3%**.

| Beat | Measured today | Source |
|---|---|---|
| Preflight | **2.35 s** | timestamped `rehearse.sh` wrapper log |
| Runner A cold build (native, fresh `CARGO_TARGET_DIR`) | **13 959 ms** = `cargo build --locked` 9.84 s + `cargo test --no-run` 3.96 s | `runner-a/build-metrics.jsonl` |
| Runner A seeded episode `stale_600ms` | **2 066 ms** | `runner-a/scenario-results.json` |
| Patch review beat | **25 ms** | rehearse log — see caveat below |
| Runner B warm build (native, fresh `CARGO_TARGET_DIR`, patch applied) | **13 034 ms** = 8.04 s + 4.84 s | `runner-b/build-metrics.jsonl` |
| Runner B gate contract tests | **4.68 s** compile, tests finish in 0.00 s | rehearse log |
| 17-scenario coverage matrix (robosuite) | **36.17 s** wall (episodes sum to 35.56 s; the 0.6 s difference is matrix setup/teardown) | rehearse log + `scenario-results.json` |
| Protected validation of runner B | **0.87 s** | rehearse log |
| **Total arc** | **76.58 s** | `1790243508.602` → `1790243585.182` |

Largest single item inside the matrix: `protocol_timeout` at **9 519 ms** — an intentional,
expected timeout, not a failure. The two native builds at ~13–14 s each are the next largest.

> **76.58 s is machine time, not stage time.** The patch-review beat spans 25 ms in the log
> because nothing human happens there. The figure and the 21.3% claim are correct **for the
> scripts**; they are not the budget for the arc as an audience experiences it. Narration,
> reading the diff aloud, and questions are on top.

**Reference points for the grid, unchanged:** the archived EC2-initiator matrix totalled
**50 906 ms** of scenario wall time (`docs/examples/ec2-runner-b`). Today's macOS matrix is
**35 560 ms**. If the talk runs on the EC2 initiator, anchor to the EC2 number and budget
90–120 s for the matrix, not today's 36 s.

---

## Honesty ledger for this run

- `command -v ib_console` → **NOT FOUND**. `REQUIRE_IB` never set (explicitly `env -u`'d on
  every preflight invocation). **0 Incredibuild builds. 0 acceleration measurements. 0
  acceleration claims.**
- No native build is labelled as an Incredibuild build anywhere in any artifact produced today.
- No ssh. No grid host touched. The simulator ran only locally.
- No number in this document is reused from an existing evidence bundle.
- `receipt.json` files that existed in this repo before today: **0**. After today: exactly one
  file literally named `receipt.json` in the repo tree
  (`evidence/build-exp-20260924T100157/build-proof/receipt.json`); the other four in that
  directory are `receipt-cold-j1.json`, `receipt-cold-j10.json`, `receipt-warm-j1.json`,
  `receipt-warm-j10.json`. Counting the two copies outside the repo under
  `outputs/evidence-live/`, there are **7 `receipt*.json` files on disk carrying 5 distinct
  receipts, 3 of them literally named `receipt.json`.** None is committed.
- **`receipt.json` files that pass `build-proof`: 0.** That number will not change on this
  machine.
- End state: HEAD `2883e19a166cedcb4075c37fb7a0cfb148b0d540` on
  `fix/proof-layer-and-public-claims`, `git status --porcelain` empty, one worktree, `.runners/`
  empty. Nothing committed, stashed, pushed, or left dirty.
