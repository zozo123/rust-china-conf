# PREFLIGHT AND RUNBOOK

**Rust China Conf · "A Million Compiles. One Robot Hour."**
Companion to `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/outputs/FINAL-TALK-E2E.md`
Status date: **2026-09-24**

**Two hosts, and every command below names which one.**

- **MAC** — the presenter laptop. macOS. Local repo `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf`. Slides, the labeled offline replay, reading exported evidence, and the offline `build-proof` refusal.
- **INITIATOR** — `qa_user@<INITIATOR_IP>`, repo `/home/qa_user/rust-china-conf-e2e`, Ubuntu 20.04 x86-64, Incredibuild 4.31.0, 1 Coordinator + 2 Helpers, CPython 3.12.14 / robosuite 1.5.2 / MuJoCo 3.9.0. Every live build and every robot episode.

**Grid hosts (VPN only):** `10.133.20.216`, `10.133.27.32`, `10.133.29.30`, `10.133.10.80`.

**Two values this document cannot supply:** `<COORDINATOR_IP>` and `IB_CLIENT_API_KEY`. Gate **P4** resolves them empirically. Everything else is exact.

> **`ib-benchmark.sh` cannot run on the Mac.** It uses `sha256sum` and `date +%s%3N`, both GNU-only. Linux initiator only.

---

## 0. Two decisions to make before anything else

### 0.1 Do not run the end-to-end wrapper on stage

`scripts/robot-demo/ec2-agentic-physical-ai.sh` is a pre-talk tool, never a stage beat. Two verified reasons:

**Cost.** `ib-benchmark.sh:183-207` runs `IB_SAMPLES=5` × {native, ib-cold, parent-seed + ib-parent-warm} = **20 full `cargo build --workspace --locked` runs** from empty target dirs, each preceded by a local-user cache wipe, each IB one followed by up to 12 × 2 s of Build History polling (`wait_for_history`, lines 84-96). At the archived EC2 build-phase time of ~21.4 s that is **10-15 minutes minimum.**

**Coupling.** Line 39 runs the benchmark, line 45 runs the behavior rehearsal, under `set -euo pipefail`. If the benchmark dies at repetition 3 of 5 — exactly what an intermittent license fault produces — **the robot safety-gate beat never runs at all.**

On stage the two beats are separate commands.

### 0.2 The `-build` suffix — pick one convention and never deviate

`ib-benchmark.sh:15` sets `OUT="$ROOT/evidence/$RUN_ID/build-proof"`. The `-build` segment that appears in every downstream path exists **only** because `ec2-agentic-physical-ai.sh:39` invokes the script as `ib-benchmark.sh "$RUN_ID-build"`.

**Convention adopted here: always invoke the benchmark as `ib-benchmark.sh "$PRERUN-build"`.**

Then the receipt is at `evidence/$PRERUN-build/build-proof/receipt.json` in every command in this document, and it matches what `evidence/README.md:22-24` already says.

**Why this matters more than it looks:** invoke it as `ib-benchmark.sh "$PRERUN"` and the receipt lands at `evidence/$PRERUN/build-proof/`, every downstream read returns `No such file`, the T-30 check exits non-zero, **and you deliver Branch B while holding a passing receipt.** Pick the convention, then verify the file exists with `ls` before you trust any exit code.

---

## 1. Environment — paste once per shell, on the INITIATOR

```bash
export REPO=/home/qa_user/rust-china-conf-e2e
cd "$REPO"
unset CARGO_TARGET_DIR                 # a stale value breaks ib-benchmark's PROOF_CLI path
export REQUIRE_IB=1                    # runner-common.sh:16 refuses to start without ib_console
export ROBOT_DEMO_BACKEND=robosuite    # real SIL, not the mock kinematic stand-in
export ROBOT_DEMO_PYTHON="$REPO/demo/robot-sim/.venv/bin/python3"   # MUST be absolute
export ROBOT_DEMO_PATCH_FILE="$REPO/demo/fallback-patch.diff"
export IB_ALLOW_CLEAR_USER_CACHE=1     # ib-benchmark.sh:29 — clears ONLY qa_user's local cache
export IB_SAMPLES=5                    # ib-benchmark.sh:26 — values below 5 are rejected
export IB_HISTORY_URL='https://<COORDINATOR_IP>:8000/api/builds?coordinatorId=<ID>&version=1.5.0'
export IB_CLIENT_API_KEY='<local secret — never on screen, never in shared scrollback>'
export IB_HISTORY_CURL_INSECURE=1      # ONLY if the coordinator serves a self-signed cert
export PRERUN='<the id you passed to ib-benchmark.sh, WITHOUT the -build suffix>'
export RUN="talk-$(date -u +%Y%m%dT%H%M%SZ)"
echo "RUN=$RUN  PRERUN=$PRERUN"
```

**`PRERUN` is in that list for a reason.** Six commands interpolate it. Unset in an interactive shell without `set -u`, it expands to empty and `"$REPO/evidence/$PRERUN-build/build-proof/receipt.json"` becomes `.../evidence/-build/...` — a `No such file` error that names the wrong problem.

**`ROBOT_DEMO_PYTHON` must be absolute, and its default is a landmine.** The runner worktree contains no `.venv`. `runner-common.sh:11` and `preflight.sh:17` are **both** `PYTHON="${ROBOT_DEMO_PYTHON:-python3}"` — only `record-gif.sh:7` defaults to the venv. Bare `python3` on the Mac is Homebrew CPython **3.14.7**, which has no robosuite (`ModuleNotFoundError`, verified 2026-09-24). An operator running `cold.sh`/`warm.sh` with `ROBOT_DEMO_BACKEND=robosuite` and no `ROBOT_DEMO_PYTHON` export hands `bridge.py` to 3.14.7 and fails **mid-episode, after the cache has been cleared**. The tell is a preflight that prints `ok python: Python 3.14.7` **and** `ok robosuite 1.5.2 / mujoco 3.9.0` on adjacent lines — the venv probe is hardcoded to the venv path, the `$PYTHON` probe is not. `ec2-agentic-physical-ai.sh` exports `REQUIRE_IB` and `ROBOT_DEMO_BACKEND` itself but **not** `ROBOT_DEMO_PYTHON` or `IB_ALLOW_CLEAR_USER_CACHE`.

**`MUJOCO_GL`: leave it UNSET on macOS.** Verified 2026-09-24 across five values: `unset`, `glfw` and `cgl` all import and render offscreen identically; **`osmesa` and `egl` both hard-fail at `import mujoco`** with `RuntimeError: invalid value for environment variable MUJOCO_GL`, before any simulation starts. Those are the two values a Linux runbook would carry over. Never copy them.

**`.env.local` precedence — CHANGED ON THIS BRANCH, verified 2026-09-24.** The old `set -a` after-the-exports behaviour is **gone**. `preflight.sh`, `runner-common.sh`, `rehearse.sh` and `ib-benchmark.sh` now all source `scripts/robot-demo/env-local.sh`, which is **fills-unset-only**: it sources `.env.local` in a child shell, filters the result, exports only names that are *not already set*, and prints `note keeping caller-provided $NAME` on stderr for anything it declined to apply. **Your exports win.** Gate **P7** still reads the file — but now to confirm the values it supplies, not to hunt for an override. (The Mac's copy holds `ISLO_SANDBOX_KEY` **and, since 2026-09-24, a `ROBOT_DEMO_PYTHON` line that the robosuite backend requires** — see P7. The initiator's cannot be reviewed from here.)

**Two panes.** PANE 1 = the beats you narrate. PANE 2 = the long-running matrix (5b/5c).

**Commit before you walk on.** `runner-common.sh:19` builds `git rev-parse HEAD^{commit}` — the **committed** revision, not your working tree.

---

## 2. Preflight checklist — T-30 minutes

One command, one unambiguous verdict. **Stop at the first FAIL and work the remedy.** Gates marked **[FAILED TODAY]** are the known-bad ones.

### P0 · `swf-cli` exists and has `build-proof` — BOTH HOSTS

**Run this first. It is the gate nobody had, and without it the branch decision itself is broken.**

```bash
# MAC
cd /Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf
cargo build --manifest-path rust/Cargo.toml --locked -p swf-cli
./rust/target/debug/swf-cli robot-demo --help | grep -q build-proof && echo CLI-PASS || echo CLI-FAIL
```
```bash
# INITIATOR
cd "$REPO/rust" && cargo build --locked -p swf-cli
"$REPO/rust/target/debug/swf-cli" robot-demo --help | grep -q build-proof && echo CLI-PASS || echo CLI-FAIL
```

- **PASS:** `CLI-PASS` on both hosts.
- **FAIL:** verified today on the Mac — the binary at `rust/target/debug/swf-cli` is dated **Sep 23 16:42**, predates the `build-proof` commit, and `swf-cli robot-demo --help` lists only `run`, `matrix`, `validate`. Invoking `build-proof` returns `error: unrecognized subcommand 'build-proof'`, exit 2.
- **Why this is stage-fatal:** the T-30 branch decision, A1, A2, B2 and abort level L4 all invoke `build-proof` at that exact path. Worse, Branch B's strongest fallback — *"no receipt at all, run it anyway and show the `No such file` error"* — instead produces a **clap usage error that reads to the audience as a broken tool, not a refusing gate.**
- **Also:** nothing on the stage path builds this binary. `runner-common.sh:26` redirects every runner build into a throwaway target dir; the only producer is `ib-benchmark.sh:48`, which sits **after** the env gates at lines 23-44. So in exactly the world Branch B is written for — the benchmark bailing on an env gate — the binary Branch B needs does not exist. **P0 is why this gate is numbered zero.**

### P1 · VPN reachability — MAC · **[FAILED TODAY]**

```bash
nc -z -G 4 <INITIATOR_IP> 22 && echo VPN-PASS || echo VPN-FAIL
```
- **PASS:** `VPN-PASS`.
- **FAIL:** reconnect GlobalProtect, re-run. Do not proceed; nothing downstream works. Persistent failure → abort level **L4**.

### P2 · SSH, and the repo at the right commit — MAC

```bash
ssh -o ConnectTimeout=5 qa_user@<INITIATOR_IP> \
  'cd /home/qa_user/rust-china-conf-e2e && git rev-parse --short HEAD && git status --porcelain | wc -l'
```
- **PASS:** prints the commit you intend to demo **and** `0` dirty files.
- **FAIL (non-zero dirty count):** the runners will build a **different revision** than you are showing. Commit or stash, re-check.

### P3 · Licensed helper cores — INITIATOR · **[FAILED TODAY]**

```bash
cd /tmp && ib_info > /tmp/ib_info.$$.txt 2>&1; grep -iE 'helper|licen|core' /tmp/ib_info.$$.txt
```
- **PASS:** each helper agent listed with a **non-zero** licensed core count.
- **FAIL:** any occurrence of `no available or licensed cores on helper machine`, or zero helper cores.
- **You must `cd /tmp` first.** `ib_info` writes `./ib_info.log` into the cwd and aborts where it cannot.
- **Necessary but not sufficient. P4 is the real gate.** (`ib_info` appears nowhere in the repo — it is an operator-side IB tool.)

### P4 · DISTRIBUTION SMOKE — the only check that proves helpers executed work — INITIATOR

```bash
cd "$REPO/rust"     # ib_profile.xml loads from $PWD; it MUST be this directory
CARGO_TARGET_DIR="$(mktemp -d /tmp/smoke.XXXX)" ib_console -c "preflight-$RUN" -f \
  --build-cache-local-user --build-cache-report-all-miss cargo build --workspace --locked
sleep 5
curl --fail -sS ${IB_HISTORY_CURL_INSECURE:+-k} -H "client-api-key: $IB_CLIENT_API_KEY" \
  "$IB_HISTORY_URL" > /tmp/hist.json
"$REPO/rust/target/debug/swf-cli" robot-demo build-history --input /tmp/hist.json --caption "preflight-$RUN"
```
- **PASS:** one JSON line with `remote_tasks >= 1` **and** `remote_core_time_s > 0`. Takes ~25-30 s.
- **`expected exactly one Build History record ... found 0`** → wrong URL or key, or history not flushed. Retry once after 10 s.
- **`... found 2`** → caption reused. Change `$RUN`. **Captions must be globally unique** (`parse_ib_history`, `main.rs:218`, requires exactly one match).
- **`remote_tasks = 0`** → **HELPERS UNLICENSED. The build ran entirely local.** `REQUIRE_IB=1` will **not** catch this — `runner-common.sh:16` and `preflight.sh:41` only test `command -v ib_console`. The live acceleration beat is dead; you are in **Branch B**.
- `rust/ib_profile.xml` requests `requested_cores=10000` with rustc `allow_remote` + `ib_cache` enabled and loads from `$PWD` with highest precedence — **run this from `$REPO/rust`, not `/tmp`.**

### P5 · Cache management tools present — INITIATOR

```bash
test -x /opt/incredibuild/management/build_avoid_cache.sh \
  && test -x /opt/incredibuild/management/show_build_cache_statistics.sh \
  && echo TOOLS-PASS || echo TOOLS-FAIL
```
- **FAIL:** `ib-benchmark.sh:39` exits 2. No receipt is possible.

### P6 · Cache statistics are parseable by this Rust verifier — INITIATOR

```bash
N=$("$REPO/rust/target/debug/swf-cli" robot-demo build-history \
      --input /tmp/hist.json --caption "preflight-$RUN" --build-number-only)
/opt/incredibuild/management/show_build_cache_statistics.sh "$N" > /tmp/cache.txt
"$REPO/rust/target/debug/swf-cli" robot-demo build-sample --mode ib-cold --repetition 1 --wall-ms 1 \
  --caption "preflight-$RUN" --source-revision parser-check \
  --history /tmp/hist.json --cache /tmp/cache.txt >/dev/null && echo CACHE-PARSE-PASS
```
- **PASS:** `CACHE-PARSE-PASS`.
- **FAIL `expected one unambiguous cache hits counter, found [...]`:** this IB version's statistics format does not match `parse_cache_counters` (`main.rs:263`). **The benchmark will die mid-run.** Do not attempt a live receipt.

### P7 · `.env.local` overrides nothing important — INITIATOR

```bash
cat "$REPO/.env.local"
```
- **PASS:** you have read every line and accept it. On **this branch** `env-local.sh` is fills-unset-only, so nothing here can override an export you already made — it can only supply a value you left unset, and it announces on stderr anything it declined to apply (`note keeping caller-provided $NAME`).
- **On the Mac (verified 2026-09-24) the file holds `ISLO_SANDBOX_KEY` *and* a `ROBOT_DEMO_PYTHON` line pointing at the venv interpreter.** That line is **required** for `ROBOT_DEMO_BACKEND=robosuite` on this machine and must **not** be deleted. This gate previously declared any `ROBOT_DEMO_PYTHON` line a FAIL; on this branch that instruction would delete the line that makes the demo work.
- **FAIL:** a `ROBOT_DEMO_BACKEND`, `REQUIRE_IB`, `ROBOT_DEMO_PATCH_FILE` or `ROBOT_DEMO_BASE_REVISION` line you did **not** intend, *and* which you have not overridden with your own export. Comment it out or accept its value knowingly.
- **`.env.local` is gitignored (`.gitignore:10`).** The Mac's `ROBOT_DEMO_PYTHON` fix is **machine-local**: a fresh clone, another operator account, or the grid host still hits the bare-`python3` default. Export it explicitly on the initiator.

### P8 · The simulator actually imports — INITIATOR

```bash
"$ROBOT_DEMO_PYTHON" -c "import robosuite, mujoco; print(robosuite.__version__, mujoco.__version__)"
```
- **PASS:** prints `1.5.2 3.9.0` (or your pinned pair).
- **FAIL:** the robosuite beat cannot run. → abort level **L3** (mock backend, with the mandatory spoken phrase) or the recorded replay.
- **Run this explicitly.** `preflight.sh:25-32` prints `warn robosuite import failed in venv` and **still exits 0** — it is not a gate for the path Beat 5 depends on.

### P9 · Repo preflight — INITIATOR

```bash
cd "$REPO" && scripts/robot-demo/preflight.sh
```
- **PASS:** `ok cargo ...`, `ok python: ...`, `ok sim venv present`, `ok robosuite ... / mujoco ...`, `ok rust/ib_profile.xml is well-formed`, `ok ib_console detected`, `== preflight complete ==`.
- **FAIL:** any line beginning `FAIL`. **Treat any `warn` on the robosuite line as a FAIL** (see P8).
- **Note** the `info islo credentials present; these scripts still use local worktrees (no remote dispatcher implemented)` line at `preflight.sh:51-52`. It is correct and it is the honest answer to any islo question.

### P10 · Dependencies fetched OUTSIDE any timed phase — INITIATOR

```bash
cd "$REPO/rust" && cargo fetch --locked && echo FETCH-PASS
```
- **PASS:** `FETCH-PASS`, no network error.
- **Do this now.** It keeps downloads out of every measured interval, and `cargo fetch` inside `ib-benchmark.sh` needs registry access from a VPN-only host — if the index is unreachable the benchmark dies before any measurement.

### P11 · No stale worktrees blocking the runners — INITIATOR

```bash
cd "$REPO" && git worktree prune && git worktree list
```
- **PASS:** only the main checkout listed. Leaked `.runners/*` worktrees come from killed runs and the scripts never prune them.

### P12 · Evidence run IDs are free — INITIATOR

```bash
ls -d "$REPO"/evidence/$RUN* 2>/dev/null && echo ID-FAIL || echo ID-PASS
```
- **PASS:** `ID-PASS`. **FAIL:** pick a new `$RUN`. **Never delete an existing evidence directory to reuse an id.**
- **Precision:** `cold.sh`/`warm.sh` reject via `runner-common.sh:22` `mkdir "$EVIDENCE"` on `evidence/<run-id>`. `ib-benchmark.sh:42` checks only `[[ ! -e "$OUT" ]]` where `OUT` is `evidence/<run-id>/build-proof` — so `evidence/<run-id>` may already exist and the benchmark proceeds. **This gate is stricter than the scripts for the benchmark and exactly right for the runners.** They are not the same rule.

### P13 · Offline fallback is armed — MAC

```bash
cd /Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf
python3 -m http.server 8765 --directory docs
```
Open `http://localhost:8765/demo/index.html?play=seeded&lang=en` — press `1`, then `→`.
- **PASS:** the seeded episode replays with **no network**. Verified: zero fetch/XHR in `docs/demo/index.html`; keys `1`/`2`/`3` and `ArrowRight` handled at lines 748-751, `?play=` at 765, `?lang=` by `assets/lang.js`.
- **Leave the tab open in a second window for the whole talk.**

### P14 · The offline evidence bundle is the ROBOSUITE one — MAC

```bash
cd /Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf
demo/robot-sim/.venv/bin/python3 demo/robot-sim/acceptance/verify_run.py \
  evidence/local-e2e-20260924T125134-runner-b
head -c 200 evidence/local-e2e-20260924T125134-runner-b/scenario-results.json
```
- **PASS:** `PROTECTED VERDICT: PASS (17 scenarios; complete coverage matrix)`, exit 0, offline, and the first record reads `"backend": "robosuite 1.5.2 / mujoco 3.9.0"`.
- **PRIMARY BUNDLE — `evidence/local-e2e-20260924T125134-runner-b`.** Produced on the Mac **2026-09-24**, robosuite 1.5.2 / MuJoCo 3.9.0, 17 scenarios, **35 560 ms** total episode wall time, artifact sha256 `72e2694ebc0e8b2cb37e3993bb770e51f7d15af0ccb4755c069b411249881a45` matching its manifest and `.sha256` sidecar. Re-verified from disk the same day. Copy at `outputs/evidence-live/robot/local-e2e-20260924T125134-runner-b` is byte-identical (`diff -r` clean).
- **Diagnostic companion — `evidence/local-e2e-20260924T125134-runner-a`** (the seeded bug). `verify_run.py … --scenario stale_600ms` → `PROTECTED VERDICT: FAIL (1 scenario(s) failed verification)` / `FAIL  stale_600ms: outcome 'cube_lifted' != expected 'rejected_stale'`, exit 1. **Say "single-scenario diagnostic" out loud** — `verify_run.py`'s own docstring says a single-scenario run is *not* matrix acceptance.
- **Second fallback — `evidence/sil-final-runner-b`** (2026-09-23, robosuite, 17 scenarios, 31 992 ms). Still valid, still verifies; use it only if today's bundle is unavailable, and name its date.
- **DO NOT use `evidence/e2e-local-20260923-194727-runner-b`.** Verified: it records `"backend": "mock (kinematic stand-in; NOT robosuite SIL)"` and 8 976 ms. Showing it while narrating robosuite/MuJoCo SIL is exactly the misrepresentation abort level L3 exists to prevent.

### P15 · The receipt exists and still verifies — INITIATOR or MAC

```bash
ls -l "$REPO/evidence/$PRERUN-build/build-proof/receipt.json"
"$REPO/rust/target/debug/swf-cli" robot-demo build-proof \
  --receipt "$REPO/evidence/$PRERUN-build/build-proof/receipt.json" --min-samples 5 \
  --distribution excluded --empty-cache-hit-floor 1
```
- **`ls` first.** A path typo and a real refusal produce the same-looking error, and confusing them costs you the branch.
- **Both flags are required for a cache-only receipt, and neither is a relaxation.** Without them the command exits 1 — that refusal is expected, not a Branch B trigger. See §3.
- **BRANCH B:** non-zero exit *with both flags*, or the file is absent.
- **BRANCH A− (where we are):** `BUILD RECEIPT CONSISTENT`. **This is a pass, not a failure.**
- **The branch is NOT decided by the printed ratio.** The receipt's `1.764x` is measured against a native build with a wiped target directory, which `ib-benchmark.sh` enforces at `native_sample() -> disposable_workspace()`. The decision is made against the preserved-target control in §3: **892 ms**, so the warm cache is 7.3x slower and this is **A−**. Exit 0 establishes a cache state and the absence of distribution; it does not establish any wall time (a receipt with fabricated 500 ms warm samples passes at `ratio=23.030x`).
- **`swf-cli` is NOT on PATH on the initiator**, contrary to the old briefing. Build it first: `cargo build --release -p swf-cli` (or `--manifest-path rust/Cargo.toml -p swf-cli` for the debug path this runbook uses).

---

## 3. T-60 minutes (or the night before): the receipt — ALREADY OBTAINED

> **A certified CACHE receipt exists on the Mac: `outputs/evidence-live/cache/receipt.json`,
> sha256 `da1b019daeaa895a1818d32154ce29280d4cf9acdbb8d165b64c78c3527062ce`, schema v2,
> 15 samples.** You do **not** need to re-run the benchmark to have a talk. Re-run it only to
> refresh the numbers on the day; budget 10-15 minutes and a new run id if you do.
>
> **The measurement beat is a CACHE beat.** Distribution is switched off at the profile
> (rustc `local_only`), `remote_tasks=0` on all 21 IB builds, and the distribution result is a
> demoted negative finding. Full write-up and stage wording: `outputs/CACHE-RESULTS.md`.

**INITIATOR — to re-run (cache-only is now the default mode):**

```bash
cd "$REPO"
export PRERUN="proof-$(date -u +%Y%m%dT%H%M%SZ)"
echo "PRERUN=$PRERUN"
export REQUIRE_IB=1                                    # without it the runner silently falls back to native cargo
time IB_ACCEL=cache-only IB_SAMPLES=5 scripts/robot-demo/ib-benchmark.sh "$PRERUN-build"
ls -l "$REPO/evidence/$PRERUN-build/build-proof/receipt.json"
```

`IB_ACCEL=cache-only` (the default) installs `rust/ib_profile.cache-only.xml` — sha256
`20d976e763a43fa623a52d41e798f1d621de5f8a6d66df0c6513a4cff94cb02d`, one attribute different from
the shipped profile: `<process filename="rustc" type="local_only">`. `IB_ACCEL=distributed` is a
clearly-labelled variant that **cannot satisfy its own gate on this grid** and fails fast in ~30 s
with that explanation. `IB_SMOKE=1` is a single-sample rehearsal that refuses to produce a receipt
at all and drops a `SMOKE-NOT-A-MEASUREMENT.txt` marker — nothing downstream may quote it.

**Verify it — the default command form REFUSES, and both extra flags are required:**
```bash
"$REPO/rust/target/debug/swf-cli" robot-demo build-proof \
  --receipt "$REPO/evidence/$PRERUN-build/build-proof/receipt.json" --min-samples 5 \
  --distribution excluded --empty-cache-hit-floor 1
```
Neither flag is a relaxation. `--distribution excluded` **demands** `remote_tasks == 0` and
`remote_core_time == 0` on every IB sample and every parent seed (one leaked task refuses the
receipt). `--empty-cache-hit-floor 1` is a corrected contract: `hits == 0` is unsatisfiable for
Rust, because cargo invokes `rustc -vV` twice and the second is served the entry the first stored.
The floor is symmetric, defaults to 0, and the receipt cannot set it.

**ALSO RUN THE CONTROL, and put it on the slide next to the receipt.** The benchmark wipes the
native target directory before every native sample, so no receipt it can produce contains the
baseline an audience will demand. Script and raw timings:
`outputs/evidence-live/cache/native-warm-control-20260924T130233Z/` (`nwcontrol.sh`, `stale.sh`).
Measured 2026-09-24 on the initiator, n=5 per mode:

```
native, persistent CARGO_TARGET_DIR, one-file patch in place   892 ms  (883-931)
native, brand-new worktree path, one shared persistent target 2778 ms  (2775-2867)
native, persistent target, no-op rebuild                        81 ms  (77-81)
IB cache-only warm, one-file change (47/52 hits)              6527 ms  (6449-6626)
```

**Expected receipt tail:**
```
BUILD RECEIPT CONSISTENT  run=<PRERUN>-build
candidate=<sha>+patch:<sha256> parent=<sha>
native          median= 11515.0ms range= 11458.. 11549ms
ib-cold         median= 16049.0ms range= 15456.. 16176ms
ib-parent-warm  median=  6527.0ms range=  6449..  6626ms
ib-cold         measured ratio=0.717x vs native; saved=-4534ms
ib-parent-warm  measured ratio=1.764x vs native; saved=4988ms
DISTRIBUTION CONTRACT: --distribution excluded (CACHE-ONLY) ...
```
**`BUILD RECEIPT CONSISTENT` does not mean the numbers are real.** `wall_ms` is corroborated by
nothing — a receipt with all five warm samples set to 500 ms passes at `ratio=23.030x`, exit 0.
Exit 0 establishes a cache state and the absence of distribution. Nothing more.

**Copy to the Mac:** `evidence/<PRERUN>-build/build-proof/{receipt.json,summary.txt,method.txt,run-order.txt,raw/,cache-report-per-task.txt}`.

### Failure handling, in order

| Symptom | Meaning | Action |
|---|---|---|
| exit 2, `set IB_ALLOW_CLEAR_USER_CACHE=1` / `IB_HISTORY_URL and IB_CLIENT_API_KEY are required` / `cache management tools are unavailable` | Env gap. **Nothing was built** — costs seconds. | Fix and rerun. **Note:** these gates sit *before* the `swf-cli` bootstrap at line 48, which is why P0 exists. |
| `refusing to reuse .../build-proof` | Run id collision | New `$PRERUN` |
| script dies ~20-30 s in, right after the **first native build** | A stale exported `CARGO_TARGET_DIR`. `PROOF_CLI` is hardcoded at `ib-benchmark.sh:21` and first used by `native_sample`'s `build-sample` call at 121-123; rep 1's rotation is (native, ib-cold, ib-parent-warm). | `unset CARGO_TARGET_DIR`, rerun. **Do not wait for a multi-minute failure — this one is fast.** |
| `Build History API never returned exactly one record for <caption>` | Coordinator API or license | Re-run **P4**. **`remote_tasks: 0` is now EXPECTED and correct** under `IB_ACCEL=cache-only` — it is what `--distribution excluded` demands. Only a *missing record* blocks the receipt. |
| `<mode> sample N has no verified remote tasks` | You verified with the **default** contract | Add `--distribution excluded`. This is the expected refusal for a cache-only receipt, not a failure. |
| `<mode> sample N was not empty-cache (hits=Some(1), floor=0)` | The unsatisfiable `hits == 0` rule | Add `--empty-cache-hit-floor 1`. The residual hit is cargo's `rustc -vV` self-hit, proven from `/etc/incredibuild/log/2026-Sep-24/local-67,71,79`. |
| `ib-parent-warm sample N has no verified cache hits` | Build cache off, or not local-user scoped | The cache-reuse claim is unavailable. The distribution claim may still stand from the ib-cold samples. |
| `expected one unambiguous cache hits counter, found [...]` | Statistics format drift — see **P6** | No receipt today |
| Out of time | — | **Use the certified receipt already on the Mac** (`outputs/evidence-live/cache/receipt.json`, sha256 `da1b019d…`) plus the control in §3. Branch B is now a fallback, not the default. Do not show 21.426 vs 21.948 as a speedup (two candidates, one sample each, uncontrolled cache, IB **522 ms slower**), and do not show the 23,173 ms `-f` run at all. |

> **Read this before you quote `CHECKED FROM RECORDS` on stage (found 2026-09-24).**
> `build-proof` reads **only the receipt's own fields.** It never opens `transcript_path`, never
> contacts Build History, never runs the cache-statistics tool. `check_clear_usable`
> (`main.rs:695-700`) validates `transcript_sha256` for **shape only** — 64 chars in `[0-9a-f]` —
> and never recomputes it. The digest *is* computed from the real file, but only at
> `build-sample` time (`load_cache_clear`, `main.rs:493`); a receipt is hand-editable afterwards.
> **Demonstrated:** a receipt with 10 hand-written IB samples, invented counters, a
> `transcript_path` of `/tmp/does-not-exist.txt` and a `transcript_sha256` of 64 zeros printed
> `BUILD RECEIPT CONSISTENT` with `ib-parent-warm measured ratio=11.948x` and **exited 0**.
> The adjacent `NOT CHECKED` paragraph is accurate and saves the tool's honesty; the
> `CHECKED FROM RECORDS` line above it is the one a reader quotes, and it over-claims.
> **On stage, read the `NOT CHECKED` paragraph aloud, or do not read either.**

> **The 2026-09-24 native-only receipt exists and is REJECTED.**
> `evidence/build-exp-20260924T100157/build-proof/receipt.json` (10 142 bytes, sha256
> `a42d40f3ea2cbf5a03e6fe3485dfc994d5529fe9d95b4c98073b2988e10ff5d2`, schema v2, 20 native
> samples) → `Error: missing benchmark mode ib-cold`, **exit 1**. That is the correct result and
> a usable Branch B artifact: a real receipt, produced by the real pipeline, that the gate
> refuses. **It is native-only. It says nothing about Incredibuild.** Its 20 samples all carry
> `mode: "native"` with a single median of **3 938 ms** — a bimodal midpoint matching no build
> anyone ran. **Never** use that median as the denominator for a future grid ratio; the four
> per-config receipts in the same directory are the honest containers.

---

## 4. On-stage runbook

### Beats 1-4 · 0:00-9:00 — no live commands

Read-only `sed` excerpts, safe on either host. Nothing executes. If the terminal is unavailable, all four beats run from slides.

```
BEAT 2   sed -n '113,150p' rust/crates/robot-safety-gate/src/lib.rs
BEAT 3   sed -n '1,30p'   scripts/robot-demo/check-patch.py
BEAT 4   sed -n '1,12p'   scripts/robot-demo/ec2-agentic-physical-ai.sh
```

### Beat 5 · 9:00-15:30 — the live robotics demonstration · INITIATOR

**Needs `ib_console` on `PATH`. Does not need licensed helpers.** Identical in all branches.

#### 5a · 9:00-10:45 · seeded failure (~30-45 s) — PANE 1
```bash
cd "$REPO" && scripts/robot-demo/cold.sh "$RUN-a"
```
Expect:
```
== runner A: local detached worktree @ <sha>; fresh build outputs ==
build provider: incredibuild; compilation-cache reuse has not been measured
== runner A: seeded stale-observation episode ==
expected seed violation reproduced: stale perception still dispatched the pickup
run-id: <RUN>-a (seeded failure; runner removed on exit)
```
**BRANCH B: add one sentence here** — *"That line says `incredibuild` because `ib_console` is on `PATH`. Hold that thought."* It plants the 19:00 payoff and closes a ten-minute uncorrected impression.

**NEVER run `validate.sh` on `$RUN-a`.** Failure record by design; the verifier is supposed to reject it.

| Failure | Action |
|---|---|
| `refusing to reuse evidence directory` | Bump `$RUN`, rerun. 20 s. |
| `REQUIRE_IB=1 but ib_console is unavailable` | Wrong host or `PATH`. → **L2** |
| `seeded regression was not reproduced` | **You are on the wrong commit.** Abort live, go to the replay, keep talking. **Do not debug on stage.** |

#### 5b · 10:45-12:45 · start B in the background, narrate the candidate

**PANE 2 — start FIRST, let it run the whole beat:**
```bash
export ROBOT_DEMO_BASE_REVISION="$(cat "$REPO/evidence/$RUN-a/agent-context/base-revision.txt")"
cd "$REPO" && scripts/robot-demo/warm.sh "$RUN-b"
```
*Safe: `warm.sh` needs only `base-revision.txt`, which `cold.sh` writes before exiting. Buys back 2:00.*

**PANE 1:**
```bash
cat evidence/$RUN-a/agent-context/work-order.md
sed -n '26,48p' demo/fallback-patch.diff
python3 scripts/robot-demo/check-patch.py demo/fallback-patch.diff
```
Expect: `candidate allowlist passed: rust/crates/robot-safety-gate/src/lib.rs`

**Say "eight added lines in the decision function."** The hunk shown adds exactly 8, but `git apply --numstat` on the whole patch reports **+11 / −20** across two hunks — hunk 1 also strips the 11-line `CONFERENCE FIXTURE` banner from the module doc comment.

**MANDATORY HONESTY LINE:** *"reviewed candidate"* / *"pre-reviewed patch."* Nothing in this repo invokes an LLM. **Never narrate it as a model writing code on stage.**

If PANE 2 died at startup, fix it here in the two minutes you already have.

#### 5c · 12:45-14:15 · B finishes — PANE 2
Expect: `candidate applied to exact base <sha>` → a second `ib_console` build → `test result: ok` for `robot-safety-gate` → 17 robosuite episodes → `run-id: <RUN>-b`.

**Timing: budget 90-120 s, not 60-90.** The archived **EC2-initiator** matrix (`docs/examples/ec2-runner-b`) totalled **50 906 ms** of scenario wall time. The **2026-09-24 macOS** matrix totalled **35 560 ms** of episode wall time / **36.17 s** of measured matrix wall, on robosuite with 0 infrastructure failures. **Anchor to the EC2 number if you are on the initiator; it is the same machine you are on.** Plan for 5c to overrun. (Longest single scenario both times: `protocol_timeout`, **9 519 ms** on the Mac — an intentional, expected timeout, not a stall.)

**Fallback:** past 14:15 → Ctrl-C, *"the simulator is not cooperating on this host — here is the rehearsal run from this morning, and note that it has its own run ID,"* and run 5d against **`evidence/local-e2e-20260924T125134-runner-b`** (2026-09-24, robosuite, 17 scenarios). `evidence/sil-final-runner-b` is the second fallback. **Do not wait on it.**

#### 5d · 14:15-15:30 · THE PROTECTED VERDICT (~5 s) — the money shot
```bash
cd "$REPO" && scripts/robot-demo/validate.sh "$RUN-b"
```
Expect 17 × `  PASS  <name>: <expected>; N labeled hold(s)` then:
```
PROTECTED VERDICT: PASS (17 scenarios; complete coverage matrix)
```

**HARD RULE: no `--scenario` on a matrix run.** `verify_run.py:207` requires the result set to equal the requested set exactly. Narrowing a 17-scenario directory **fails with a coverage mismatch on the one slide that is supposed to be your verdict.** (The `--scenario fresh_lift` example at `README.md:55` / `README.en.md:65` applies only to a single-scenario evidence directory.)

| Failure | Meaning | Action |
|---|---|---|
| `artifact digest differs from manifest` | The binary was rebuilt after export | *"The digest check just caught a rebuild — that is the check doing its job."* Show the rehearsal evidence. **Recoverable and on-message.** |
| `scenario coverage mismatch` | You passed `--scenario` | Drop it, rerun. 5 s. |

### Beat 6 · 15:30-21:00 · MEASUREMENT

> **BRANCH IS A−, AND IT IS A CACHE BEAT.** Lead with the cache. Demote distribution to an
> honest negative finding. **Payoff slide: `892 ms`** — one agentic-loop iteration with plain
> `cargo` and a preserved target directory (n=5, 883-931), against **6,527 ms** through
> Incredibuild's warm Build Cache and **11,515 ms** from scratch. Stage wording:
> `outputs/CACHE-RESULTS.md` §6.

**Branch decided at T-30 by P15. Do not decide it on stage.**

**Shared live sequence — INITIATOR, ~45 s. NOTE THE TWO CHANGES FROM THE OLD SCRIPT:**
```bash
cd "$REPO/rust"
# 1. NO -f. `-f` is --force-remote: it forces every remotable task onto helpers and idles the
#    initiator's own 4 cores. It produced the discredited "2.01x slower" number.
# 2. A FIXED target path, contents wiped -- the Build Cache key includes the rustc output path,
#    so a mktemp target guarantees ~0% reuse by construction. That is why every earlier run
#    showed 1 hit out of 52.
cp "$REPO/rust/ib_profile.cache-only.xml" "$REPO/rust/ib_profile.xml"   # rustc type="local_only"
export CARGO_TARGET_DIR=/tmp/stage-fixed-target
rm -rf "$CARGO_TARGET_DIR" && mkdir -p "$CARGO_TARGET_DIR"
ib_console -c "stage-$RUN" \
  --build-cache-local-user --build-cache-report-all-miss cargo build --workspace --locked
# the number that matters is the per-task Build Cache report, not Build History:
grep -c '^HIT:' <the ib_hm.log path ib_console just printed>     # expect 52 on a repeat run
curl --fail -sS ${IB_HISTORY_CURL_INSECURE:+-k} -H "client-api-key: $IB_CLIENT_API_KEY" \
  "$IB_HISTORY_URL" > /tmp/stage.json
"$REPO/rust/target/debug/swf-cli" robot-demo build-history --input /tmp/stage.json --caption "stage-$RUN"
```
Expect `{"build_number":N,"remote_tasks":0,"local_tasks":N,"remote_core_time_s":0.0}` —
**zero is the expected and correct answer**, because distribution is off at the profile. Say so
before you show it, or the room will read it as a failure.

**`--build-cache-local-user` is inert for rustc** (it selects the C/C++ ccache store at
`/etc/incredibuild/cache/build_avoid/<user>.<uid>`). The rustc store is
`/etc/incredibuild/cache/build_cache/shared`. `/ib/mnt/fscache` (28 KB used) is the
remote-execution file service and is **not** the build cache — never report it as cache size.

**⚠️ The cache clear is MACHINE-WIDE.** `build_avoid_cache.sh:127` runs
`rm -rf /etc/incredibuild/cache/build_cache/shared/*` unconditionally for every scope argument.
Anyone else on this initiator loses their entire rustc Build Cache when you run a cold sample.
The verifier's own stdout still prints `cache scope ... local-user`; that line is false.

**NEVER re-run the curl with a different caption to get a nicer answer.** Captions must be globally unique or `build-history` fails with `found 2`.

**Say "on our grid," not "in this room."** The grid is on the corporate VPN, on a different continent from the venue.

**This build is not the receipt's configuration and cannot be correlated to Beat 5's builds.** `runner-common.sh:62-63` invokes bare `ib_console cargo build --locked` with **no caption**, no `-f`, no cache flags and no `--workspace`, and `parse_ib_history` matches by caption. So Beat 5's builds are permanently uncorrelatable to any counter. Never say *"the same `ib_console` path you just saw measured."*

#### Branch A− (where we are)
```bash
cat "$REPO/evidence/$PRERUN-build/build-proof/method.txt"       # A1, 15:30
cat "$REPO/evidence/$PRERUN-build/build-proof/run-order.txt"    # A1
cat "$REPO/evidence/$PRERUN-build/build-proof/summary.txt"      # A2, 17:00 -- the receipt
cat "$REPO/evidence/$PRERUN-build/build-proof/cache-report-per-task.txt"   # A2 -- the HIT counts
cat outputs/evidence-live/cache/native-warm-control-*/results.txt          # A2 -- THE CONTROL
sed -n '403,477p' rust/crates/swf-cli/src/main.rs               # A3, 19:00 (re-derive the offsets)
```
**Show the control on the same slide as the receipt.** Receipt: `ib-parent-warm 6,527 ms,
ratio 1.764x vs native`. Control: the same one-file patch with the target directory preserved is
**892 ms**, so the cache is 7.3x slower; a no-op rebuild is **81 ms**, so 52/52 full reuse
(3,706 ms) is 46x slower. Both configurations compile the same three crates
(`robot-safety-gate`, `swf-app`, `swf-cli`).

**Then the one thing Incredibuild wins, and it is not speed** (`stale.sh` in the control bundle):
one shared target directory across two worktrees, candidate's changed file backdated — cargo
returns in **83 ms having compiled nothing**, shipping an rlib byte-identical to the parent's
(`0f0c4b82e21d5aec`). After `touch`: 2,797 ms and a different rlib. cargo's fingerprint is
mtime-based; Incredibuild's key is command-line/content-based and cannot fail this way.

#### Branch B
```bash
# B1, 15:30
sed -n '172,207p' scripts/robot-demo/ib-benchmark.sh
# B2, 17:00 — the refusal
"$REPO/rust/target/debug/swf-cli" robot-demo build-proof \
  --receipt "$REPO/evidence/$PRERUN-build/build-proof/receipt.json" --min-samples 5 ; echo "exit=$?"
# then the shared live sequence above — expect remote_tasks: 0
# B3, 19:00 — the measured conditions, then the two inference bugs
sed -n '403,477p' rust/crates/swf-cli/src/main.rs
sed -n '53,78p'   scripts/robot-demo/runner-common.sh
sed -n '41,49p'   scripts/robot-demo/preflight.sh
sed -n '820,832p' rust/crates/swf-cli/src/main.rs
```

**`sed` ranges are corrected and load-bearing:**

- **STALE LINE NUMBERS — re-derive on the day.** Schema v2 moved `validate_build_proof` to `main.rs:845`, bails through `:958`. Use `845,965p` and confirm with `grep -n "fn validate_build_proof" main.rs` before reciting anything.
- **`53,78p`, not `53,60p`.** Line 57 is `mode=incredibuild`; the payoff — `"ib": provider == "incredibuild"` — is at line **76**, inside the heredoc. The short range leaves half your argument unsupported on screen.
- **`820,832p` is STALE.** That hardcoding was the second inference bug and it is **fixed as of 2026-09-24**: cache facts are parsed from a `cache-clear.sh` transcript, never constructed. To show the fix live, use `grep -n "transcript" main.rs | head` or the guard test `cache_state_is_never_asserted_by_construction`.

**There is no `BUILD PROOF FAIL` banner.** `build-receipt` (`main.rs:809-834`) writes a receipt unconditionally; `build-proof` is the gate and bails with an anyhow error on stderr plus a non-zero exit. **The stage artifact is the bail text itself.**

**Expected B2 text — corrected:**
```
ib-cold sample 1 has no verified remote tasks
ib-parent-warm sample 2 has no verified cache hits
missing benchmark mode native
Error: reading .../receipt.json
Caused by:
    No such file or directory (os error 2)
```
Two corrections to what was previously circulated: a mode absent **entirely** fails at the `with_context` on `main.rs:463` with `missing benchmark mode native` — `has 0 sample(s), require at least 5` is only reachable when the mode is *present* with too few samples. And the file-missing case prints **two anyhow blocks**, not one line.

**Bind the spoken line to what is actually on screen:**

| Screen | You may say |
|---|---|
| `... has no verified remote tasks` | *"Every Incredibuild sample in that receipt reported zero remote tasks. The verifier will not certify a distribution claim from a build that did not distribute."* |
| `No such file` / `missing benchmark mode` | *"We never produced the experiment. The gate will not accept an absent experiment as a neutral result either."* |
| live `remote_tasks: 0` | *"Zero. The helpers did no work for this build."* |
| live `remote_tasks` unexpectedly > 0 | *"That one build distributed — but one build is not the experiment, and I am not going to convert a single sample into a claim."* **Then continue with B3 unchanged. Do not improvise a speedup.** |

**Do not scripted-assert the vendor root cause.** *"The coordinator cannot load its license keypair"* is an operator report, not something on the screen, and it is a public diagnosis of your own employer's product. Branch B is entered on *any* non-zero exit — missing env vars, a stats parse failure, a history timeout, the benchmark never having run — and in several of those worlds no build ran at all. **Keep it for Q&A, hedged:** *"our operators are still working the cause."*

### Beat 7 · 21:00-23:30 · What the evidence keeps

**INITIATOR:**
```bash
ls -1 evidence/$RUN-b
cat evidence/$RUN-b/artifact/swf-cli.sha256
sha256sum evidence/$RUN-b/artifact/swf-cli
```
**MAC** (abort level L4) — same three, last line becomes:
```bash
shasum -a 256 evidence/local-e2e-20260924T125134-runner-b/artifact/swf-cli
# expect 72e2694ebc0e8b2cb37e3993bb770e51f7d15af0ccb4755c069b411249881a45
```
**`sha256sum` is GNU-only.** Name the host before you type. Every other beat names its host; this one and the L4 fallback previously did not, and under L4 the Mac is the only machine left.

`runner-common.sh:87` writes `swf-cli.sha256`; `verify_run.py:198-202` recomputes and compares against both the manifest digest and that file.

**DO NOT SAY 88/88.** Archived revision, older verifier. Today's verifier says seventeen scenarios.
**10/10 is fine and current** — 8 `#[test]` in `tests/contract.rs` + 2 in `src/lib.rs`, verified by count.
**Never present the archived `f358e898…` digest as verified** — that executable is not committed. Use the binary exported live in 5c.

### Beat 8 · 23:30-25:00 · Close

No command. Say *"compilation that is meant to be reused"*, not *"reusable compilation"* — every `build-metrics` record carries `"cache_reuse_verified": false`.

---

## 5. Failure playbook — what to say and do

### What you MAY say

- *"Implemented, and here is the controlled CACHE receipt"* — the certified receipt `da1b019d…` covers this today. **`remote_tasks` is 0 and that is correct**: distribution is off at the profile.
- *"Measured, and on this workload it was not faster"* — Branch A−. Legitimate, and stronger than it feels. **Back it with the 892 ms control, not just the receipt.**
- *"The build cache works — forty-seven of fifty-two compilations served from cache after a one-file change, fifty-two of fifty-two on identical source. Those are Incredibuild's own counters."* Measured, from the per-task Build Cache report.
- *"An empty cache is slower than no cache: sixteen seconds against eleven and a half."* Measured, n=5.
- *"Incredibuild's cache key is the command line and the content, so it cannot serve you a stale artifact. Cargo's fingerprint is mtime-based, and it can."* Measured, with a deterministic repro.
- *"Distribution is not the story here, and the reason is structural: rustc is one process per crate, and this graph is deep, not wide."* Structural claim, measured counters behind it.
- Disposable git worktrees and fresh Cargo target dirs on an EC2-backed Linux initiator. **Removing a workspace is not destroying a machine.**
- Software-in-the-loop, robosuite/MuJoCo, segment-level authorization.
- *"No trace in this evidence pack contains a dispatch the gate did not permit."*
- *"10/10 — 8 contract plus 2 unit, still current."*

### What you MAY NOT say

- **88/88.** Stale — archived revision, older verifier.
- **21.426 s vs 21.948 s as acceleration.** Different candidates, one sample each, uncontrolled cache, and the IB one was **522 ms SLOWER**.
- **The 23,173 ms / "2.01x slower" distribution figure, in either direction.** It was produced with `-f` = `--force-remote`, which forced every remotable task onto two helpers and idled the initiator's own four cores (`maxInitiatorCores=0`). It measures a handicapped configuration, not Incredibuild.
- **"1.76x faster" or "3.13x faster" without the baseline in the same breath.** Both are measured against a native build starting from an empty target directory. Preserved-target native is **892 ms** (one-file change) and **81 ms** (no-op). Say the baseline or do not say the ratio.
- **"Certified" as shorthand for "the speed-up is proven."** `wall_ms` is corroborated by nothing; a receipt with fabricated 500 ms warm samples passes at exit 0 with `ratio=23.030x`. The gate certifies a cache state and the absence of distribution.
- **Any quantitative distribution conclusion from the without-`-f` run.** It is **n=1**, a smoke result.
- **"Disposable workspaces, reusable compilation"** as an unqualified claim. Reuse requires the target *path* to be pinned; a pinned path is not disposable, and keeping the directory's contents instead is ~80x cheaper.
- **"The cache is shared across the grid."** `BuildCache.ServiceURL` is unset and `BuildCacheService.SizeLimit` is 0 — the store is machine-local and cross-machine reuse is **unmeasured**.
- **"The clear only touches my own cache."** `build_avoid_cache.sh:127` `rm -rf`s the shared rustc store unconditionally for every scope. It is machine-wide.
- **`/ib/mnt/fscache` as the build cache size.** It is the remote-execution file service, 28 KB used. The rustc store is `/etc/incredibuild/cache/build_cache/shared`.
- **"Our committed evidence bundles say `ib: true`."** Only 2 of 8 do; they use an old schema with no `build_provider` field and were **not produced by the runner on screen.** All six bundles under `evidence/` say `ib: false`.
- **"Every build we ran on an unlicensed grid was recorded as Incredibuild."** Nothing on disk establishes this. **Use the subjunctive:** *"a build that executed entirely locally **would** be recorded as an Incredibuild build; nothing in this evidence format could tell you otherwise."*
- ~~"There is no inference anywhere in that path."~~ **No longer banned — true as of 2026-09-24.** The three conditions were hardcoded and are now transcribed from a cache-clear transcript. Say it as found-and-fixed, never as always-true.
- **"An entirely fictional speedup."** An inferring receipt fed local builds gives a ratio near **1.000**, not a speedup. Say *"a number that meant nothing."*
- **"Incredibuild 4.31.0, one coordinator and two helpers" as recorded evidence.** No manifest in the repo contains an `incredibuild` field of any kind. The topology is prose at `docs/examples/README.en.md:30` and a hardcoded string at `docs/demo/index.html:386`.
- **islo as working.** Planned provider, 14 tracked files, zero working code. An `ISLO_SANDBOX_KEY` changes nothing — `preflight.sh:51-52` says so on stdout.
- **"A coding agent just wrote this fix."** No LLM is wired in anywhere.
- **The archived `f358e898…` sha256 as a verified artifact.** Not committed.
- **Hardware-in-the-loop, physical robot validation, trained vision, continuous per-control-step supervision.**
- **A 2-scenario live run as the full matrix.** If you shorten it, say *"rehearsal coverage"* out loud.
- **"The arm cannot move on stale perception."** The gate is advisory over self-reported protocol messages.
- **"In this room"** about the grid. It is on the VPN, on another continent.

### If it breaks live

| Break | Say | Do |
|---|---|---|
| 5a `REQUIRE_IB` error | *"Native build, no acceleration claimed"* — **before** the screen says `build provider: native` | `unset REQUIRE_IB`, rerun |
| 5c matrix stalls past 14:15 | *"The simulator is not cooperating on this host — here is the rehearsal run, with its own run ID"* | Ctrl-C, 5d against `evidence/local-e2e-20260924T125134-runner-b` (then `sil-final-runner-b`) |
| 5d digest mismatch | *"The digest check just caught a rebuild — that is the check doing its job"* | Show rehearsal evidence. On-message. |
| 5d coverage mismatch | nothing — fix it in 5 s | Drop `--scenario`, rerun |
| Stage build hangs > 45 s | *"The grid is busy; the controlled run from this morning stands"* | Ctrl-C, stay on the receipt |
| `remote_tasks: 0` in Branch A | *"The helper licenses are not loading right now, so that build ran locally. The measurements on screen are from a licensed run and I am not going to claim this one."* | Continue to A3. You have not lost the branch. |
| `remote_tasks` > 0 in Branch B | *"That one build distributed — but one build is not the experiment."* | Continue B3 unchanged |
| No receipt at all | *"Our committed evidence today proves the integration path, not a speedup. Two historical observations, 21.4 and 21.9 seconds, one sample each, different candidates, uncontrolled cache — the second was 522 milliseconds slower. I am not presenting that as acceleration. The controlled experiment is implemented in `ib-benchmark.sh` and gated by a Rust verifier that fails closed on missing telemetry. It has not produced a passing receipt yet."* | Spend the recovered time on Beat 5, which is fully real |
| Robosuite import fails | *"Mock — kinematic stand-in, NOT robosuite SIL."* Say that exact phrase. | `export ROBOT_DEMO_BACKEND=mock` |

---

## 6. Abort ladder

| Level | Trigger | Response |
|---|---|---|
| **L0** | All green | Run as written. |
| **L1** | `remote_tasks = 0` | **You are in Branch B.** Branch B needs no helpers. Beat 5 is unchanged. |
| **L2** | `ib_console` missing / grid down | `unset REQUIRE_IB`. Beat 5 runs on native cargo. **Say "native build, no acceleration claimed" before `build_candidate` prints `build provider: native` on screen.** |
| **L3** | robosuite import fails | `export ROBOT_DEMO_BACKEND=mock`. Every manifest and result line will read `mock (kinematic stand-in; NOT robosuite SIL)`. **Say that phrase out loud in 5a.** |
| **L4** | VPN or SSH down | **Mac only.** `python3 -m http.server 8765 --directory docs`, then `http://localhost:8765/demo/index.html?play=seeded&lang=en`, keys `1` / `→` / `2` / `3`. **Say "recorded replay" in the first sentence.** Beat 6 Branch B still runs offline from the copied receipt — **requires the Mac-side `swf-cli` build from P0.** Beat 7 uses **`evidence/local-e2e-20260924T125134-runner-b`** (robosuite, 17 scenarios, 2026-09-24; `sil-final-runner-b` is the second fallback), **not** `e2e-local-...-runner-b` (mock). |
| **L5** | Out of time | Drop 5b's patch read-through. Keep 5a, 5c, 5d. **The protected verdict is the only beat that cannot be replaced by a slide.** |

---

## 7. One-page card

```
BEFORE YOU LEAVE THE HOTEL
  [ ] cargo build -p swf-cli on BOTH hosts; grep build-proof in --help   (P0)
  [ ] PRERUN exported; ls the receipt file, do not trust exit codes alone
  [ ] ib-benchmark.sh was invoked as "$PRERUN-build"                      (§0.2)
  [ ] cat .env.local on the initiator — read every line; export ROBOT_DEMO_PYTHON
      yourself, the Mac's .env.local line is gitignored and does not travel  (P7)
  [ ] landing.html:145-146 fixed, or do not show the site at all          (B6)
  [ ] git status clean on the initiator; git worktree prune run
  [ ] cargo fetch --locked done
  [ ] localhost:8765 replay tab open in a second window
  [ ] evidence/local-e2e-20260924T125134-runner-b verified offline on Mac  (P14)
  [ ] MUJOCO_GL unset on the Mac (osmesa/egl hard-fail at import mujoco)
  [ ] branch decided: A- (CACHE beat). Verify with BOTH flags:
      --distribution excluded --empty-cache-hit-floor 1   (bare form REFUSES, by design)
  [ ] the 892 ms CONTROL is on the slide next to the receipt
      outputs/evidence-live/cache/native-warm-control-*/results.txt
  [ ] ib_profile.cache-only.xml installed; NO -f anywhere; FIXED target path

THE PAYOFF SLIDE — ONE NUMBER
  892 ms   one agentic-loop iteration, plain cargo, target directory preserved
           n=5, range 883-931, initiator, 2026-09-24
  vs  6,527 ms  Incredibuild warm Build Cache, 47/52 hits, remote_tasks=0   (7.3x slower)
  vs 11,515 ms  native from an empty target directory                      (12.9x slower)
  IB cold cache 16,049 ms = 39% SLOWER than no cache at all

HOSTS
  Beats 1-4        either        read-only sed
  Beat  5          INITIATOR     cold.sh / check-patch.py / warm.sh / validate.sh
  Beat  6          INITIATOR     build-proof + ib_console + build-history
                   (MAC for Branch B offline: build-proof is a pure file read)
  Beat  7          INITIATOR     sha256sum      MAC: shasum -a 256
  Beat  8          slide only

NEVER
  validate.sh on $RUN-a           - failure record by design
  validate.sh --scenario on a matrix run
  ib-benchmark.sh or the e2e wrapper on stage
  a reused ib_console caption
  a reused evidence run id
  -f / --force-remote      - it idles the initiator's 4 cores; source of the bad 23,173 ms
  a mktemp CARGO_TARGET_DIR for a cache demo - the key includes the output path: 1 hit / 52
  "1.76x" or "3.13x" without saying the baseline had an empty target directory
  "certified" as shorthand for "the speed-up is proven" - wall_ms is corroborated by nothing
  any distribution number from the without-`-f` run - it is n=1
  88/88   f358e898...   "a coding agent wrote this"   "in this room"
  (REMOVED from the banned list 2026-09-24 — now true; see the proof-layer fix)

SAY WITHOUT BEING ASKED
  reviewed candidate, not a live model                              Beat 3
  workspaces removed, not machines                                  Beat 4
  the gate is advisory over self-reported protocol messages         Beat 2
  measured, and the cache was not faster than keeping the target dir Beat 6
  remote_tasks is ZERO on purpose - distribution is off at the profile Beat 6
  the cache cannot serve a stale artifact; cargo's mtime check can    Beat 6
  cross-machine cache sharing is UNMEASURED (ServiceURL unset)        Beat 6
  the cache clear is machine-wide, not per-user                       Beat 6
  implemented, not measured                                         Beat 6B
  the title is motivation, not a measured conversion                Beat 1
```
