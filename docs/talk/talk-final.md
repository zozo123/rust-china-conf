# FINAL TALK — END TO END

**"A Million Compiles. One Robot Hour." · Rust China Conf**
Repo: `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf` (branch `main`, commit `a3c35f9`, working tree carries four applied `docs/talk/` edits)
Public site: `zozo123.github.io/rust-china-conf/`
Status date: **2026-09-24**

The operational half of this talk lives in a separate file so it can be held in one hand on stage:
`/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/outputs/PREFLIGHT-AND-RUNBOOK.md`

---

## 1. TL;DR — the honest state of the evidence

**The robotics half of this talk is real, live, and reproducible. The acceleration half is not measured, and may not be measured by talk time.**

What is demonstrably true today, verified against files on disk:

| Claim | Status | Backing |
|---|---|---|
| A Rust crate decides whether each motion segment may be dispatched, and this revision omits the freshness rule | **TRUE** | `rust/crates/robot-safety-gate/src/lib.rs:113-149`; the omission is literally `let _ = (age_ms, policy);` at line 147 |
| A seeded 600 ms stale observation still dispatches the pickup | **TRUE** | `cold.sh` asserts it and refuses to export otherwise |
| A candidate patch may touch exactly one file | **TRUE** | `scripts/robot-demo/check-patch.py`, `ALLOWED_PATH` is a single constant |
| A 17-scenario protected matrix runs and passes against a freshly exported executable | **TRUE** | `demo/robot-sim/config/coverage-matrix.json` = 5 placements × 3 freshness + `estop_descend` + `protocol_timeout`; `verify_run.py:238` prints `PROTECTED VERDICT: PASS (17 scenarios; complete coverage matrix)` |
| The verdict is bound by sha256 to the exported binary | **TRUE** | `verify_run.py:197-203` recomputes and compares against both the manifest digest and the `.sha256` file |
| The contract suite is 10/10 (8 contract + 2 unit) | **TRUE, AND CURRENT** | `grep -c '#\[test\]'` → `tests/contract.rs` 8, `src/lib.rs` 2 |
| Incredibuild accelerates this build | **MEASURED 2026-09-24 — THE ANSWER IS NO** | Cache-only, 21 IB builds, n=5/mode, gated receipt `da1b019d…`. IB warm one-file rebuild **6,527 ms**; plain `cargo` with the target directory preserved does the same rebuild in **892 ms**. Full write-up: `CACHE-RESULTS.md` |
| The verifier "fails closed with no inference anywhere" | **STILL PARTLY FALSE — do not say this** | The *construction* tautology is fixed: cache facts are transcribed per sample from a `cache-clear.sh` transcript (sha256-retained), guarded by `cache_state_is_never_asserted_by_construction` and `schema_one_receipts_are_rejected_because_their_attestations_were_literals`. **But at proof time nothing is re-read.** Measured 2026-09-24: a forged receipt with invented IB counters, `transcript_path=/tmp/does-not-exist.txt` and a `transcript_sha256` of 64 zeros printed `BUILD RECEIPT CONSISTENT … ratio=11.948x`, exit 0. Say **"it fails closed on what is missing; it cannot detect what is fabricated"** |
| A native-only receipt can be validated | **FALSE — and correctly so** | `validate_build_proof` (`main.rs:941-948`) iterates `["native","ib-cold","ib-parent-warm"]`. The real 2026-09-24 receipt → `Error: missing benchmark mode ib-cold`, exit 1 |
| The 17-scenario matrix and both protected verdicts run offline on the Mac | **TRUE, MEASURED 2026-09-24** | `evidence/local-e2e-20260924T125134-runner-{a,b}`; full arc 76.58 s, 21.3% of the 360 s budget |
| 88/88 verifier checks | **STALE** | today's verifier prints 17 scenarios, never 88 |
| Helpers executed compilation work | **MEASURED — ZERO, BY DESIGN AND THEN BY DEFAULT** | The cache-only run declares rustc `local_only`: `remote_tasks=0` and `remote_core_time=0` on all 21 IB builds, from Build History. Separately, with `allow_remote` and **without** `-f`, this grid distributed nothing either (`numberOfRemoteTasks=0`) — but that is **n=1** and may not be quoted as a conclusion |

### The central problem, stated plainly — RESOLVED 2026-09-24, and not in our favour

**The acceleration half is now measured. Incredibuild's Build Cache works on this workload and is slower than plain `cargo`.**

The headline for the measurement beat is one number: **892 ms** — the median cost of one agentic-loop iteration (apply the one-file candidate patch, rebuild) using plain `cargo` with the target directory preserved, n=5, range 883–931 ms. The same iteration through Incredibuild's Build Cache costs **6,527 ms** (n=5, 47 of 52 compilations served from cache, `remote_tasks=0`), and **11,515 ms** from an empty target directory (n=5).

| Comparison | Result |
|---|---|
| IB warm one-file (6,527 ms) vs native with target preserved (892 ms) | **IB 7.3x SLOWER** |
| IB warm one-file vs native, brand-new worktree + one shared target (2,778 ms) | **IB 2.35x SLOWER** |
| IB full reuse (3,706 ms, 52/52 hits) vs native no-op rebuild (81 ms) | **IB 46x SLOWER** |
| IB cold cache (16,049 ms) vs native from scratch (11,515 ms) | **IB 39% SLOWER** |
| IB warm one-file vs native **from scratch** (11,515 ms) | IB 1.76x faster — *the certified number, handicapped baseline* |

**Why the certified 1.76x is not the headline.** `ib-benchmark.sh`'s `native_sample()` calls `disposable_workspace()`, which `rm -rf`s the native target directory before *every* native sample, while Incredibuild is handed a persistent store that survives across samples and is deliberately seeded with the parent revision. Every ratio in the receipt inherits that asymmetry. The control that settles it — plain `cargo` with the target directory kept — had never been run; it was run on 2026-09-24 at 13:02 UTC on the initiator (`evidence-live/cache/native-warm-control-20260924T130233Z/`) and it reverses the sign.

**It is the same work on both sides.** cargo's own output on the 892 ms rebuild reads `Compiling robot-safety-gate / swf-app / swf-cli` — exactly the three crates that MISS in Incredibuild's 47/52 warm build. Both recompile three crates. cargo serves the other 49 by checking they are on disk (81 ms); Incredibuild serves 47 of them by unpacking cache tars (~5.6 s).

**The old 21.426 / 21.948 / 522 ms pair is still not a measurement** and is still the fastest way to lose the room. It is now superseded rather than merely disclaimed.

**And the earlier "Incredibuild is 2.01x slower" distribution number is worse than useless.** It was produced with `-f`, which is `--force-remote`: every remotable task was *forced* onto two m5.large helpers while the initiator's own four cores sat idle (`maxInitiatorCores=0`). That is the wall time of a deliberately handicapped configuration, not a measurement of Incredibuild. **Do not quote it in any branch.**

### The missing asset — OBTAINED, with a precise scope

A controlled benchmark receipt exists and is certified: `evidence-live/cache/receipt.json`, sha256 `da1b019daeaa895a1818d32154ce29280d4cf9acdbb8d165b64c78c3527062ce`, schema v2, 15 samples, 5 per mode, rotating order.

```
swf-cli robot-demo build-proof --receipt <path> --min-samples 5 \
        --distribution excluded --empty-cache-hit-floor 1
-> BUILD RECEIPT CONSISTENT, exit 0
```

The two flags are **verifier-supplied and neither is a weakened threshold.** `--distribution excluded` is a demand in the opposite direction (`remote_tasks == 0` and `remote_core_time == 0` on every IB sample *and* every parent seed; one leaked task refuses the receipt). `--empty-cache-hit-floor 1` is a corrected contract: the old `hits == 0` rule is factually unsatisfiable for Rust, because cargo invokes `rustc -vV` twice and the second invocation is served the entry the first just stored — proven from `/etc/incredibuild/log/2026-Sep-24/local-67,71,79`. The floor is symmetric and defaults to 0, and the receipt cannot set it.

**What exit 0 does NOT cover, and this must be said on stage:** not the wall times. `wall_ms` is corroborated by nothing — a receipt with all five warm samples set to 500 ms still prints `BUILD RECEIPT CONSISTENT` at `ratio=23.030x`, exit 0. The certification establishes **a cache state and the absence of distribution**, and nothing about the speed numbers. It also does not cover the 3.13x full-reuse figure, which the three-mode schema cannot carry and which was therefore never gated. Full scope, including the one false line the verdict still prints (`cache scope … local-user`, when the clear is machine-wide), is in `CACHE-RESULTS.md` §5.

### Two new findings that change the plan

1. **The archived `ib: true` records were not produced by the runner that will be on stage.** `docs/examples/ec2-runner-{a,b}/build-metrics.jsonl` use an old schema — `{"phase","runner","wall_ms","cache_namespace":".runners/a/ib-cache","ib":true}` — with **no** `build_provider`, `sandbox_provider` or `cache_reuse_verified` field. The committed `runner-common.sh:74-78` writes all three and writes `cache_namespace: null`. All six bundles under `evidence/` say `ib: false`. So the only two IB-labelled measurements in the repo come from a script revision that is not the one being projected, and the semantics of their `ib` flag cannot be audited from this tree. This is a *stronger* objection than "one sample per mode" and an IB-literate reviewer will find it first.

2. **The three unfalsifiable conditions are FIXED (2026-09-24).** They were real: `BuildProof` was constructed with `cache_scope`/`cache_cleared_before_each_cold_sample`/`cache_cleared_before_each_parent_seed` as literals, so three of the eight advertised fail-closed conditions could never fire. It was proven by feeding the verifier a fabricated receipt with no Incredibuild involvement and no cache ever cleared: it printed `BUILD PROOF PASS ... measured ratio=2.999x vs native; saved=14000ms`, exit 0. **Schema v2 removes it:** cache facts are parsed out of a `scripts/robot-demo/cache-clear.sh` transcript whose sha256 is retained, nothing is defaulted and nothing is inferred; a sample with no transcript is *unknown*, not clean; v1 receipts are rejected outright because their attestations were literals. Two regression tests fail if anyone puts it back. **Branch B narrates this as found-and-fixed** — a second inference bug, in the proof layer itself, caught by the same discipline the talk argues for. That is a stronger beat than the sentence it replaced.

3. **The fix did not go far enough, and that was measured on 2026-09-24 — this is now the strongest beat in Branch B.** Schema v2 stopped the receipt *asserting* its cache state at construction. It did not make `build-proof` *re-read* anything. `validate_build_proof` reads only the receipt's own fields: it never opens `transcript_path`, never contacts Build History, never runs the cache-statistics tool, and `check_clear_usable` (`main.rs:695-700`) checks `transcript_sha256` for **shape only** (64 chars in `[0-9a-f]`) without ever recomputing it. The digest *is* genuinely computed from the file — at `build-sample` time (`load_cache_clear`, `main.rs:493`) — but a receipt is a hand-editable JSON file afterwards. **Proven by construction:** a receipt of 5 real native samples plus 10 hand-written IB samples with `remote_tasks=412`, `remote_core_time_s=880.5`, `transcript_path: "/tmp/does-not-exist.txt"` and `transcript_sha256` = 64 zeros printed `BUILD RECEIPT CONSISTENT`, `ib-parent-warm measured ratio=11.948x vs native; saved=7116ms`, **exit 0**. It required inventing nothing but JSON fields.

   **The repo defect to fix before the stage:** `print_build_proof` (`main.rs:996-1005`) prints `CHECKED FROM RECORDS: … one Build History record per caption reporting success; … cold-cache hits==0 and warm-cache hits>0 from the cache-statistics tool; every IB build preceded by a transcribed local-user cache clear that exited 0.` **None of that is checked at proof time**, and on the forged receipt the whole line printed verbatim over fabricated counters. The adjacent `NOT CHECKED` paragraph is accurate and saves the tool's honesty — but `CHECKED FROM RECORDS` is the line a reader quotes. Either recompute the digests and re-read the retained responses at proof time, or reword it to `CHECKED FROM THIS RECEIPT'S OWN FIELDS`. **Narrate this as the third inference bug, found by the same discipline that found the first two.**

---

## 2. Blocker ledger

Ordered by "what breaks if this is not resolved."

| # | Blocker | Who unblocks | Cost if unresolved |
|---|---|---|---|
| **B1** | GlobalProtect VPN disconnected; grid hosts `10.133.20.216 / .27.32 / .29.30 / .10.80` unreachable | Operator (you) | Total. No live beat at all. Falls to abort level L4 — Mac-only recorded replay. |
| **B2** | IB coordinator cannot load its license keypair; helper cores unlicensed (`no available or licensed cores on helper machine`) | Grid operators / IB support | No receipt, ever. Talk goes to **Branch B**. Robotics beat is unaffected. |
| **B3** | `rust/target/debug/swf-cli` on the Mac is from **Sep 23 16:42** and predates `build-proof`. `swf-cli robot-demo --help` lists only `run`, `matrix`, `validate`. | You, in 15 seconds: `cargo build --manifest-path rust/Cargo.toml --locked -p swf-cli` | **Stage-fatal for both branches.** The branch decision, A1, A2, B2 and abort level L4 all invoke `build-proof` at that exact path. It returns `error: unrecognized subcommand 'build-proof'`, exit 2 — a clap usage error that reads to the audience as a broken tool, not a refusing gate. |
| **B4** | Receipt path is off by a `-build` suffix. `ib-benchmark.sh:15` writes `$ROOT/evidence/$RUN_ID/build-proof`; the suffix exists **only** because `ec2-agentic-physical-ai.sh:39` calls it as `ib-benchmark.sh "$RUN_ID-build"`. | You, by picking one convention | **Worst failure in the whole plan:** a receipt that PASSED reads as `No such file`, the T-30 check exits non-zero, and you deliver Branch B while holding a passing receipt. **Resolution adopted here: always invoke `ib-benchmark.sh "$PRERUN-build"`,** so `evidence/$PRERUN-build/build-proof/receipt.json` is correct everywhere and matches `evidence/README.md`. |
| **B5** | `$PRERUN` is interpolated by six commands and defined by none | You | `evidence/-build/...`, a `No such file` that names the wrong problem. |
| **B6** | Public site contradicts the talk: `landing.html:145-146` hardcodes `21.426`/`21.948` as **template literals outside `copy.json`**, so `build.py --check` exits 0 while both public pages keep the stale pair; `docs/demo/loop.{html,en.html}` print `(compile stage distributed)`, `88 / 88 PASS` as a green climax frame, and `instance gone` | You, before the talk | Beat 8 sends the room to a site that asserts exactly what Beat 7 forbids. |
| **B7** | `.env.local` is sourced with `set -a` **after** your exports, in `preflight.sh:6` and `runner-common.sh:9` | You: `cat .env.local` at preflight | A gitignored file on the initiator can silently flip `ROBOT_DEMO_BACKEND`, `REQUIRE_IB` or `ROBOT_DEMO_PYTHON`. Nothing in any checklist looks at it. (The local copy holds only `ISLO_SANDBOX_KEY` — the initiator's cannot be reviewed from here.) |
| **B8** | Nothing on the stage path builds `$REPO/rust/target/debug/swf-cli`. `runner-common.sh:26` redirects every runner build into a throwaway target dir; the only producer is `ib-benchmark.sh:48`, which sits **after** the env gates at lines 23-44 | You, at preflight P0 | In exactly the world Branch B is written for — benchmark bailed on an env gate — the binary Branch B's centrepiece needs does not exist. |

**Everything B3 through B8 is fixable in under twenty minutes and costs nothing. B1 and B2 are outside your control. Plan for them.**

---

## 3. The talk, minute by minute

Eight beats, 25:00 on the clock, Q&A separate. **Beats 1-5, 7 and 8 are shared verbatim** — 18:00, plus most of the branch beat's closing argument. **Only Beat 6 (15:30-21:00) branches**, plus one swapped sentence in the close. Nothing before 15:30 promises a speedup, by design; the measurement beat is the only place either branch makes a performance statement.

**Three branches, not two.** The T-30 decision is in §3.6.

```
  BEAT 1  0:00– 2:00  2:00  The robot succeeded                 SHARED
  BEAT 2  2:00– 4:30  2:30  What Rust decides                   SHARED
  BEAT 3  4:30– 6:30  2:00  A candidate cannot redefine passing SHARED
  BEAT 4  6:30– 9:00  2:30  Disposable workspaces; Rust owns proof  SHARED
  BEAT 5  9:00–15:30  6:30  The live robotics demonstration     SHARED
  BEAT 6 15:30–21:00  5:30  MEASUREMENT — A+ / A− / B           BRANCHES
  BEAT 7 21:00–23:30  2:30  What the evidence keeps             SHARED
  BEAT 8 23:30–25:00  1:30  Close                    SHARED but one sentence
                     -----
                     25:00
```

For a 20-minute cut: take 2:00 off Beat 5 (drop 5b's work-order read, start `warm.sh` earlier), 1:00 off Beat 4 (drop the islo caption), 1:00 off Beat 7, 1:00 off Beat 2. **Never cut Beat 6 and never cut 5d.**

---

### BEAT 1 · 0:00–2:00 · The robot succeeded — SHARED

No command. Slide only: one frame of the lift, two verdicts side by side.

> "The robot lifted the cube. The simulator reported success. Would you merge it?
>
> Now look at the timestamps. Every motion segment was authorized from an observation that was 600 milliseconds old. Our policy allows 250. The task succeeded. The contract failed. Both of those sentences are true, and only one of them was visible."

Then the scope sentence, immediately, before anyone can assume otherwise:

> "This is a simulated Panda arm in robosuite and MuJoCo. Simulation is how we make the mistake repeatable. There is no physical hardware in this talk, and the title is our motivation — not a measured conversion between compiles and robot hours."

**Fallback:** entirely verbal. If the deck fails, say the four sentences over a black screen.

---

### BEAT 2 · 2:00–4:30 · What Rust decides — SHARED

```
sed -n '113,150p' rust/crates/robot-safety-gate/src/lib.rs
```
*(Read-only. Runs anywhere, including the Mac. Verified: the doc comment begins at 113, `pub fn decide` is at 119, the function ends at 149.)*

> "The controller proposes approach, descend, grasp, lift. A Rust crate decides whether each segment may be dispatched. The evaluation order is contractual, and there are exactly three rules:
> 1. simulated emergency stop — precedes everything;
> 2. timestamp validity — a capture time after the current simulation time is invalid;
> 3. freshness — an observation older than the policy threshold is rejected as `StalePerception`.
>
> This revision implements rule 1 and rule 2 and deliberately omits rule 3. You are looking at the omission: `let _ = (age_ms, policy);`. It compiles. It passes `cargo build`. It lifts the cube. **A successful compilation does not establish a behavioral property. Neither does a successful lift.** The contract test is the thing that asks the additional question."

**Then the boundary sentence. Mandatory. Say it slowly, and say the second half — it is the question you otherwise cannot answer:**

> "One important boundary. Authorization happens at segment dispatch. A segment contains multiple control steps, and hold steps are explicitly unguarded — the source says so at the top of the file. 250 milliseconds is an illustrative policy value, not a hardware safety limit.
>
> And be precise about what this proves. `decide` is a pure function over protocol messages the bridge sends it. What the evidence establishes is that **no trace in this evidence pack contains a dispatch the gate did not permit** — not that the arm cannot physically move on stale perception. A controller that moved the arm and never sent a proposal would not be caught by anything in this crate. That is a real limit of the design and I would rather state it than be asked."

*(Why this matters: `swf-app/src/session.rs:333` errors with `task dispatched without permission` only when the bridge **reports** `dispatched: true` for a non-permitted action. The gate is advisory over self-reported protocol messages. Without this sentence, Beat 5d's "Zero stale dispatches" invites an objection you have no answer to.)*

**Fallback:** the same source is on the slide as a static excerpt. Nothing here executes.

---

### BEAT 3 · 4:30–6:30 · A candidate does not get to redefine acceptance — SHARED

```
sed -n '1,30p' scripts/robot-demo/check-patch.py
```

> "When something proposes a fix — a person, a tool, a model — it may touch exactly one file: `rust/crates/robot-safety-gate/src/lib.rs`. Not the threshold. Not the fixtures. Not the acceptance checks. Not the runner. Not the evidence generator. Mode changes, renames, new files, binary hunks and empty patches are rejected outright. **A candidate does not get to redefine what passing means, and it does not get to edit the thing that judges it.**"

**HONESTY LINE — mandatory. Say it here, unprompted, rather than being asked later:**

> "To be exact about what this repository does and does not do: the checked-in rehearsal applies a **reviewed patch**. Nothing in these scripts invokes a language model. The interface accepts a patch from an external agent; today I am showing you the reviewed candidate."

> *[If running a live external agent: show it explicitly and cap the attempt at approximately 45 seconds. On timeout, switch to the reviewed patch and say that you are switching.]*

**This is the single likeliest place in the talk to get caught.** Never narrate the fallback patch as a model writing code on stage.

---

### BEAT 4 · 6:30–9:00 · Disposable workspaces, and why Rust owns the proof path — SHARED

```
sed -n '1,12p' scripts/robot-demo/ec2-agentic-physical-ai.sh
```

> "Every candidate needs fresh validation, so every candidate gets a workspace that did not exist a minute ago: a detached git worktree and a fresh Cargo target directory on a Linux host, removed on exit. Be precise about the word sandbox — **we remove workspaces, not machines.** This is not VM isolation and we do not provision or destroy EC2 instances.
>
> That split is the interesting part. The workspace is disposable. Test verdicts must be earned again for this candidate. But eligible compilation work is exactly the thing that *should* survive the workspace — that is what a build service is for, and here that is Incredibuild: Cargo running under `ib_console` with a rustc profile that marks compilation remote-eligible.
>
> And because this is a Rust conference, Rust owns the proof path too. `swf-cli` — not a spreadsheet, not the vendor's dashboard — parses the coordinator's Build History response and the cache statistics, and it fails closed. It rejects the run if any Incredibuild sample has zero remote tasks, zero remote core time, ambiguous counters, or, on the parent-warmed path, zero cache hits. Python is only the robosuite adapter. It proposes motion. It cannot authorize it and it cannot certify the build."

Provider caption — under 30 seconds, do not elaborate:

> "Islo is the intended future provider of the disposable-execution role. It has no working code in this repository; today's execution is the EC2-backed grid."

*(For accuracy in your own head: islo is named in **14** tracked files, not four. The claim "planned provider, zero working code" is correct in all of them.)*

**Fallback:** read-only. Behind schedule? Cut the islo sentence entirely.

---

### BEAT 5 · 9:00–15:30 · The live robotics demonstration — SHARED, both branches

**This beat is the spine of the talk. It needs `ib_console` on `PATH`; it does not need licensed helpers.** It runs identically in every branch, which is why Branch B is not a weaker talk — 6:30 of live demonstration is untouched by the grid's license state.

Two panes. **PANE 1** = what you narrate. **PANE 2** = the long-running matrix.

#### 5a · 9:00–10:45 — the seeded failure, live

```
scripts/robot-demo/cold.sh "$RUN-a"
```
Expect, in order:
```
== runner A: local detached worktree @ <sha>; fresh build outputs ==
build provider: incredibuild; compilation-cache reuse has not been measured
== runner A: seeded stale-observation episode ==
expected seed violation reproduced: stale perception still dispatched the pickup
run-id: <RUN>-a (seeded failure; runner removed on exit)
```

Over the build: *"Fresh worktree, fresh target directory, the committed revision — not my laptop's working tree."*

On the assertion line: *"The cube lifted, and the freshness contract was violated. The script asserts that the regression reproduced; if it had not, it refuses to export a misleading failure packet. **A demo that cannot fail is not a demo.**"*

**BRANCH B ONLY — add one line, and plant it early:**
> "One thing on that second line. It says `incredibuild` because `ib_console` is on `PATH`. Hold that thought. I come back to it in about ten minutes."

*(Without this, a false impression stands uncorrected from 9:00 until the debunk at 19:00. With it, the debunk has a setup.)*

**NEVER run `validate.sh` on `$RUN-a`.** Runner A's evidence is a failure record by design; the verifier is supposed to reject it.

Fallbacks: `refusing to reuse evidence directory` → bump `$RUN`, rerun, 20 s. `REQUIRE_IB=1 but ib_console is unavailable` → `unset REQUIRE_IB`, rerun, and say **"native build, no acceleration claimed"** *before* the audience reads `build provider: native`. `seeded regression was not reproduced` → you are on the wrong commit; abort live, go to the recorded replay, do not debug on stage.

#### 5b · 10:45–12:45 — the bounded candidate, while B builds behind it

**PANE 2 — start this first and let it run for the whole beat:**
```
export ROBOT_DEMO_BASE_REVISION="$(cat "$REPO/evidence/$RUN-a/agent-context/base-revision.txt")"
scripts/robot-demo/warm.sh "$RUN-b"
```
*(Safe: `warm.sh` needs only `base-revision.txt`, which `cold.sh` writes before exiting. This buys back 2:00.)*

**PANE 1 — narrate over it:**
```
cat evidence/$RUN-a/agent-context/work-order.md
sed -n '26,48p' demo/fallback-patch.diff
python3 scripts/robot-demo/check-patch.py demo/fallback-patch.diff
```
Expect: `candidate allowlist passed: rust/crates/robot-safety-gate/src/lib.rs`

> "Here is the work order the candidate received, here is the diff, and here is the allowlist accepting it. **Eight added lines in the decision function** — rule 3, restored, with the boundary pinned: exactly the threshold permits, one millisecond beyond rejects."

**Say "in the decision function."** The hunk at lines 26-48 adds exactly 8 lines, but `git apply --numstat` on the whole patch reports **+11 / −20** across two hunks — hunk 1 also strips the 11-line `CONFERENCE FIXTURE — SEEDED REGRESSION` banner from the module doc comment. If anyone runs numstat, the bare "eight lines" disagrees. Consider showing hunk 1 too: removing the seeded-regression banner is itself part of the candidate, and the allowlist permits doc-comment rewrites as readily as logic changes.

#### 5c · 12:45–14:15 — B finishes: fresh build from the recorded base, full matrix

Expect in PANE 2: `candidate applied to exact base <sha>` → a second `ib_console` build → `test result: ok` for `robot-safety-gate` → 17 robosuite episodes → `run-id: <RUN>-b`.

> "That build started from the exact revision runner A recorded, in a workspace that did not exist three minutes ago, and it just ran the full protected matrix: five cube placements times three observation ages, plus a simulated stop and a protocol timeout. Seventeen episodes."

**Timing reference — use the right anchor.** On the EC2 initiator the archived 17-episode matrix totalled **50 906 ms** of scenario wall time (`docs/examples/ec2-runner-b`). The **2026-09-24 macOS** run (`evidence/local-e2e-20260924T125134-runner-b`, robosuite, 0 infrastructure failures) totalled **35 560 ms** of episode wall / **36.17 s** of matrix wall; longest single scenario `protocol_timeout` at **9 519 ms**, an intentional expected timeout. **Budget 90–120 s** on the initiator including MuJoCo init, and plan for 5c to overrun its 1:30 window. For reference, the *entire* rehearsal arc on the Mac — preflight, runner A, patch, runner B, protected validation — took **76.58 s, 21.3% of the 360 s budget** (machine time only; the patch-review beat is 25 ms in the log because nothing human happens there).

**Fallback:** if the matrix has not finished by 14:15, Ctrl-C, say *"the simulator is not cooperating on this host — here is the rehearsal run from this morning, and note that it has its own run ID,"* and run 5d against **`evidence/local-e2e-20260924T125134-runner-b`** (2026-09-24, robosuite, 17 scenarios, verified offline on the Mac the same day). `evidence/sil-final-runner-b` is the second fallback. Do not wait on it.

#### 5d · 14:15–15:30 — THE PROTECTED VERDICT

```
scripts/robot-demo/validate.sh "$RUN-b"
```
Expect 17 lines of `  PASS  <name>: <expected>; N labeled hold(s)` then:
```
PROTECTED VERDICT: PASS (17 scenarios; complete coverage matrix)
```

> "Seventeen scenarios. Ten lifts, five stale rejections, one emergency stop, one timeout. Zero stale dispatches in this trace — and the fresh task still completes, which is the half that matters, because a gate that rejects everything is not a fix. And that verdict is bound by sha256 to the executable this run produced, not to a binary I brought with me in my bag."

**HARD RULE: no `--scenario` flag on a matrix run.** `verify_run.py:207` requires the result set to equal the requested set *exactly*; narrowing a 17-scenario directory fails with a coverage mismatch on the one slide that is supposed to be your verdict. (The `--scenario fresh_lift` example lives at `README.md:55` and `README.en.md:65` and applies only to an evidence directory containing exactly that one scenario.)

Fallbacks: `artifact digest differs from manifest` → the binary was rebuilt after export. Say *"the digest check just caught a rebuild — that is the check doing its job,"* and show the rehearsal evidence. This is recoverable and on-message. `scenario coverage mismatch` → you passed `--scenario`; drop it, 5 s.

---

### BEAT 6 · 15:30–21:00 · MEASUREMENT — the branch

> ## THE BRANCH IS DECIDED: **A−, and it is a CACHE beat, not a distribution beat.**
>
> A certified receipt exists (`evidence-live/cache/receipt.json`, sha256 `da1b019d…`). Distribution is switched off at the profile — rustc `local_only` — so every number is the Build Cache and nothing else. The cache demonstrably works (47/52 and 52/52 hits from Incredibuild's own counters) **and is slower than plain `cargo` with the target directory preserved.** Lead with the cache; demote distribution to an honest negative finding (`CACHE-RESULTS.md` §4).
>
> **The one number on the payoff slide: `892 ms`.** One agentic-loop iteration with plain `cargo` and a preserved target directory (n=5, 883–931 ms). Against Incredibuild's warm Build Cache: **6,527 ms**. Against a from-scratch build: **11,515 ms**.

**The decision command, run at T-30 (the default form REFUSES — both flags are required and neither is a relaxation):**
```
"$REPO/rust/target/debug/swf-cli" robot-demo build-proof \
  --receipt "$REPO/evidence/$PRERUN-build/build-proof/receipt.json" --min-samples 5 \
  --distribution excluded --empty-cache-hit-floor 1
```

| Result | Branch |
|---|---|
| `BUILD RECEIPT CONSISTENT` **and** the preserved-target control beats the IB warm median | **A− (WHERE WE ARE)** — receipt obtained, IB not faster |
| `BUILD RECEIPT CONSISTENT` **and** IB beats the *preserved-target* baseline | **A+** — not observed on this grid, in any configuration |
| Non-zero exit, or no receipt file | **B** — no measurement |

**A+ now requires beating the right baseline.** The old A+ test — `measured ratio ≥ 1.000` — is satisfied by the receipt (1.764x) and means only that Incredibuild beat a native build starting from an empty target directory. That is not the question an audience asks. **The A+/A− decision is made against the 892 ms preserved-target control, and on this grid there is no branch where the cache wins: keep the target directory → native wins 7.3x; discard the workspace and relocate the target directory → native wins 2.35x; fresh machine → the IB cache is empty and starts at 16,049 ms against 11,515 ms; share the cache between machines → not possible here (`BuildCache.ServiceURL` is unset).**

**A+ and A− are genuinely different talks and you must decide which you are in before you walk on.** `print_build_proof` (`main.rs:498-507`) computes `ratio = native / measured` and `saved = native − measured` and prints them unconditionally once validation passes. **A receipt in which Incredibuild is slower still exits 0 and still prints `BUILD PROOF PASS`,** with a ratio like `0.918x` and a negative `saved`. `BUILD PROOF PASS` means *the telemetry was verified*, not *IB was faster*. Given that the only IB-labelled observation in this repo is 522 ms slower, **A− is a live outcome** and improvising in front of a ratio that reads `0.9xx` is not a plan.

**Do not run `ib-benchmark.sh` or `ec2-agentic-physical-ai.sh` on stage.** The benchmark is 20 full `cargo build --workspace --locked` runs plus up to 24 s of Build History polling per IB sample — 10-15 minutes. And `ec2-agentic-physical-ai.sh:39/45` chains the benchmark then `rehearse.sh` under `set -euo pipefail`, so a benchmark failure at repetition 3 of 5 — exactly what an intermittent license fault produces — would kill the robotics beat entirely. **The receipt is a pre-talk artifact. The stage shows it.**

---

#### BRANCH A+ · receipt obtained, Incredibuild faster

*Arc: we told you it was implemented and not measured. Here is the measurement, and here is what it does not license.*

**A1 · 15:30–17:00 — the experiment, before any number**
```
cat "$REPO/evidence/$PRERUN-build/build-proof/method.txt"
cat "$REPO/evidence/$PRERUN-build/build-proof/run-order.txt"
```
> "Before the number, the method — because you cannot check a number you cannot reproduce. One candidate identity, fixed: base revision plus the sha256 of the patch. Three modes: native Cargo; Incredibuild with my user's local cache explicitly cleared first; and Incredibuild after that cache has been populated by building the **parent** revision only. Five samples per mode. Rotating order, so drift in the grid does not land on one mode. `cargo fetch` runs before any timed interval, so no measurement contains a download. Every build gets its own empty target directory."

**A2 · 17:00–19:00 — the number, and one live build**
```
cat "$REPO/evidence/$PRERUN-build/build-proof/summary.txt"
```
```
BUILD PROOF PASS  run=<PRERUN>
candidate=<sha>+patch:<sha256> parent=<sha>
native          median=  ...ms range= ...ms
ib-cold         median=  ...ms range= ...ms
ib-parent-warm  median=  ...ms range= ...ms
ib-cold         measured ratio=N.NNNx vs native; saved=...ms
ib-parent-warm  measured ratio=N.NNNx vs native; saved=...ms
distribution verified for every IB sample; parent-warmed cache hits verified for every warm sample
```
> "Medians and ranges, both modes, against native on the same candidate. And the last line is the one I care about: the Rust verifier computed those ratios only after every Incredibuild sample proved remote work and every warm sample proved cache hits."

Then live, ~45 s, from `$REPO/rust`:
```
# CACHE-ONLY, and DELIBERATELY at a FIXED target path so the cache can serve it.
# -f is --force-remote: it forces every remotable task onto helpers and idles the
# initiator's own 4 cores. It produced the discredited 2.01x-slower number. NEVER use it here.
# --build-cache-local-user is INERT for rustc (it selects the C/C++ ccache store); kept only
# because the receipt's argv carries it.
cp "$REPO/rust/ib_profile.cache-only.xml" "$REPO/rust/ib_profile.xml"   # rustc type="local_only"
export CARGO_TARGET_DIR=/tmp/stage-fixed-target && rm -rf "$CARGO_TARGET_DIR" && mkdir -p "$CARGO_TARGET_DIR"
ib_console -c "stage-$RUN" \
  --build-cache-local-user --build-cache-report-all-miss cargo build --workspace --locked
curl --fail -sS ${IB_HISTORY_CURL_INSECURE:+-k} -H "client-api-key: $IB_CLIENT_API_KEY" \
  "$IB_HISTORY_URL" > /tmp/stage.json
"$REPO/rust/target/debug/swf-cli" robot-demo build-history --input /tmp/stage.json --caption "stage-$RUN"
```
Expect `{"build_number":N,"remote_tasks":0,"local_tasks":N,"remote_core_time_s":0.0}` — **zero is the expected and correct answer here**, and the per-task Build Cache report is where the real number lives:

```
grep -c '^HIT:' <the ib_hm.log path ib_console prints>     # expect 52 on a repeat run at this path
```

> "`remote_tasks` is zero, and that is the point: I switched distribution off at the profile — one word, `allow_remote` to `local_only` — so everything you are about to see is the build cache and nothing else. Fifty-two of fifty-two compilations served from cache. Those are Incredibuild's counters, not my stopwatch."

*(Say "on our grid," not "in this room." The grid is on the corporate VPN, reached over GlobalProtect, on a different continent from the venue. Someone will know.)*

Fallbacks: build hangs past 45 s → Ctrl-C, *"the grid is busy; the controlled run from this morning stands."* Cost zero — the receipt is the evidence. `remote_tasks: 0` or `found 0 records` → *"The helper licenses are not loading right now, so that build ran locally. The measurements on screen are from a licensed run and I am not going to claim this one."* You have not lost the branch. **Never re-run the curl with a different caption to get a nicer answer** — captions must be globally unique or `build-history` fails with `found 2`.

**A3 · 19:00–21:00 — what the number does not license** *(~70% shared with A− and B)*
```
sed -n '403,477p' rust/crates/swf-cli/src/main.rs     # validate_build_proof, the measured conditions
```
**The line numbers below are STALE — schema v2 moved everything.** `validate_build_proof` now begins at `main.rs:845` and its bails run to `:958`. Use `845,965p`. Re-derive exact offsets on the day with `grep -n "fn validate_build_proof" main.rs`; do not recite line numbers you have not just checked.

> "Here is why you should believe the previous slide, and it is not because I ran it. This function refuses to print a ratio if any Incredibuild sample has zero remote tasks or zero remote core time; if the telemetry is incomplete or ambiguous; if any cold sample has a non-zero cache-hit count; if any warm sample has a zero cache-hit count; if a sample's source revision differs from the candidate; if repetition numbers repeat; or if any mode has fewer than five samples. Missing telemetry is a failure. Ambiguous telemetry is a failure."

**Then the honest limits, said as strength, not hedging:**
> "What we measured is `cargo build --workspace --locked`. Not provisioning, not checkout, not agent latency, not tests, not simulation, not export — and to be precise, **not the build you watched in the robotics beat**, which is a different and smaller command. A compilation improvement may or may not dominate the complete candidate cycle. I am not converting it into cost or robot-hours, because we did not measure those quantities. The title of this talk is our motivation. It is not a result."

**You may now say "there is no inference anywhere in that path" — as of 2026-09-24 it is true, and only because it was found to be false and fixed.** The stronger line: *"Three fields in this receipt used to be asserted by the tool that wrote them. We caught it, and the fix is a test that fails if anyone puts it back."* Do not claim it was always true.

---

#### BRANCH A− · receipt obtained, Incredibuild not faster

*Arc: the experiment ran, and it did not say what we hoped. Here is the number anyway, and here is why showing it is the point.*

**Everything in A1 is unchanged** — the method beat is the same, and it matters more here, not less.

**A2− · 17:00–19:00 — the number that did not go our way**

Show the receipt, then show the control the receipt does not contain. **Both, in this order.**

```
native          median= 11515.0ms range= 11458.. 11549ms
ib-cold         median= 16049.0ms range= 15456.. 16176ms      1 hit / 52    remote_tasks=0
ib-parent-warm  median=  6527.0ms range=  6449..  6626ms     47 hits / 52   remote_tasks=0
ib-cold         measured ratio=0.717x vs native; saved=-4534ms
ib-parent-warm  measured ratio=1.764x vs native; saved=4988ms
```
```
native, persistent CARGO_TARGET_DIR, same one-file patch applied in place:
  892 ms   (n=5, range 883-931)        -> IB's warm cache is 7.3x SLOWER
native, BRAND-NEW worktree path per candidate, one shared persistent target:
  2778 ms  (n=5, range 2775-2867)      -> IB's warm cache is 2.35x SLOWER
native, persistent target, nothing changed (no-op rebuild):
  81 ms    (n=5, range 77-81)          -> IB's 52/52 full reuse (3706 ms) is 46x SLOWER
```

> "The certified ratio says Incredibuild's cache is 1.76x faster than native. That is true, and it is measured against a native build that starts from an empty target directory every single time, because the benchmark wipes it. So I ran the control the benchmark never ran: keep the target directory, apply the same one-file patch, rebuild. Eight hundred and ninety-two milliseconds. The cache takes six and a half seconds to do the same thing.
>
> And it is the same work. Cargo says `Compiling robot-safety-gate, swf-app, swf-cli` — exactly the three crates that miss in Incredibuild's warm build. Both do three compiles. The difference is the other forty-nine: cargo checks they are on disk in eighty-one milliseconds; the cache unpacks forty-seven tarballs in five and a half seconds.
>
> An empty cache is worse than no cache: sixteen seconds against eleven and a half. That is the price of writing fifty-one entries, and it belongs on the slide next to everything else."

**Then give Incredibuild the one thing it genuinely wins, because it is real and it is the better argument for this talk** — see `CACHE-RESULTS.md` §6.7:

```
# one shared target directory, two worktrees, candidate's changed file with an older mtime
candidate build: 83 ms, 0 crates compiled, rlib byte-identical to the parent (0f0c4b82e21d5aec)
after `touch`:   2797 ms, rlib 91c4838950d76ff0
```

> "cargo's fingerprint is mtime-based. Share one target directory across worktrees and it will hand you yesterday's binary and call it a success — eighty-three milliseconds, zero crates compiled, byte-identical to the parent. Incredibuild's cache key is the command line and the content, and it cannot fail this way. For a talk about a gate that decides whether a robot may move, *the fast path cannot silently serve you a stale binary* is a stronger property than any ratio."

> "Five samples per mode, one candidate, rotating order, downloads outside timing, and every Incredibuild sample carrying verified remote work. And the answer is: **on this workload, at this scale, Incredibuild was not faster.** The ratio is right there and I am not going to round it.
>
> I want to be clear about what that is and is not. It is not a claim that distributed compilation does not work — this is one crate graph on one grid with two helpers, and the distribution telemetry verified, which means the work genuinely went out and came back. What it says is that for *this* workload the coordination cost was not repaid. That is a scale question, and the honest version of the answer is that we now have an instrument that can tell us where the crossover is, and one data point on the wrong side of it."

Run the same live `ib_console` + Build Cache report sequence as A+ — and the point has changed: `remote_tasks` is **zero by design**, so the cache is cleanly isolated, the reuse is real (52/52), and the *benefit* is what did not appear. **Do not run the `-f` form. Do not narrate distribution at all beyond the structural explanation below:** rustc is one process per crate, this graph is deep rather than wide (45 packages, ~52 units), and proc-macro crates and build scripts must be built *and run* on the host, so they sit on the critical path and cannot be distributed. There is very little width to sell to helpers. That is a measured structural finding, not a vendor criticism — and the without-`-f` distribution result behind it is **n=1**, so state nothing quantitative from it.

**A3− · 19:00–21:00 — identical to A3**, with one added closing sentence:
> "I could have shown you the 21.4 and 21.9 second numbers from an earlier run, or the 23-second one from the run where I had accidentally forced every task onto two remote helpers and idled the four cores under my own desk. They are two different candidates with one sample each and an uncontrolled cache, and the Incredibuild one was 522 milliseconds slower — so they would have told you nothing, badly. Five samples per mode with verified telemetry tells you something real, even when it is not the something you wanted."

**Do not apologize in A−.** A measured negative on a verified instrument is a better talk than a measured positive on an unverified one, and this audience knows it.

---

#### BRANCH B · no receipt

*Arc: the proof gate failed closed. It refused to certify. The refusal is the result.*

**This is not a consolation prize.** It is the only version of this talk in which you demonstrate the instrument rather than its output, and for a Rust audience that is the more interesting object. Deliver it that way.

**B1 · 15:30–17:00 — the experiment we built, stated as a commitment**
```
sed -n '172,207p' scripts/robot-demo/ib-benchmark.sh
```
> "Here is the experiment, and I am going to describe it before I tell you what happened, because otherwise you would rightly assume I designed the criteria after seeing the data. One candidate identity: base revision plus the sha256 of the patch. Three modes: native Cargo; Incredibuild with my user's local cache explicitly cleared; Incredibuild after that cache is populated by building the parent revision only. Five samples per mode, minimum. Rotating order — you can see the rotation right there, three orderings cycling across repetitions. `cargo fetch` outside every timed interval. A fresh target directory per build. Twenty builds, fifteen minutes, and it is not something you run on stage."

Then the turn:
> "That is what we committed to measure. **Now: we do not have the measurement.**"

**B2 · 17:00–19:00 — the refusal, live**
```
"$REPO/rust/target/debug/swf-cli" robot-demo build-proof \
  --receipt "$REPO/evidence/$PRERUN-build/build-proof/receipt.json" --min-samples 5 ; echo "exit=$?"
```
There is **no `BUILD PROOF FAIL` banner.** The stage artifact is the bail text on stderr plus a non-zero exit. Expect one of:
```
ib-cold sample 1 has no verified remote tasks
ib-parent-warm sample 2 has no verified cache hits
missing benchmark mode native
Error: reading .../receipt.json
Caused by:
    No such file or directory (os error 2)
```
*(Two corrections to the expected text: a mode that is absent entirely fails at the `with_context` on `main.rs:463` with `missing benchmark mode native` — the `has 0 sample(s), require at least 5` message is only reachable when the mode is present with too few samples. And the file-missing case prints two anyhow blocks, not one line.)*

**Say only what is on the screen. The line is conditional:**

- If the screen says **`... has no verified remote tasks`**:
  > "That is our own performance verifier, run on our own data, thirty seconds ago. It will not print a speedup. Every Incredibuild sample in that receipt reported zero remote tasks — the helpers did no work for those builds. The verifier will not certify a distribution claim from a build that did not distribute, so there is no number on this slide, and there is not going to be one."

- If the screen says **`No such file`** or **`missing benchmark mode`**:
  > "That is our own performance verifier, and what it is telling you is that we never produced the experiment. It will not accept an absent experiment as a neutral result either. There is no number on this slide."

**Do not scripted-assert the vendor root cause.** "The coordinator cannot load its license keypair" is an operator report, not something on the screen, and it is a public diagnosis of your own employer's product at a conference. Branch B is entered on *any* non-zero exit — including missing env vars, a cache-statistics parse failure, a history timeout, or the benchmark never having been run — and in several of those worlds no build ran at all. **Keep the license explanation for Q&A, hedged:** *"our operators are still working the cause."*

Then live, ~45 s — show the ground truth rather than asserting it. Same `ib_console` + `build-history` sequence as A2. Expect `{"build_number":N,"remote_tasks":0,"local_tasks":N,"remote_core_time_s":0.0}`

> "`remote_tasks`: zero. That is the whole story, and I would rather show you the zero than tell you about a number you cannot reproduce."

Fallbacks: **VPN or SSH down** → run the identical `build-proof` command on the Mac against the copied receipt. It is a pure file read and needs no grid — *provided you built `swf-cli` on the Mac* (see blocker B3). Skip the live `ib_console` half. **`remote_tasks` unexpectedly non-zero** → you are in Branch A's *fallback*, not Branch A. Say *"that one build distributed — but one build is not the experiment, and I am not going to convert a single sample into a claim,"* and continue with B3 unchanged. **Do not improvise a speedup.**

**B3 · 19:00–21:00 — why a gate that fails closed is the result**
```
sed -n '403,477p' rust/crates/swf-cli/src/main.rs
```
> "This is the function that just refused. It will not print a ratio if any Incredibuild sample has zero remote tasks or zero remote core time; if the telemetry is incomplete or ambiguous; if any cold sample has a non-zero cache-hit count; if any warm sample has a zero cache-hit count; if a sample built a different revision than the candidate; if repetition numbers repeat; or if any mode has fewer than five samples. Every one of those is a bail, not a warning."

**THE TURN — the strongest ninety seconds available to this talk. Two bugs, both found in our own instrumentation.**
```
sed -n '53,78p' scripts/robot-demo/runner-common.sh
sed -n '41,49p' scripts/robot-demo/preflight.sh
sed -n '820,832p' rust/crates/swf-cli/src/main.rs
```
*(Use `53,78p`, not `53,60p`. Line 57 is `mode=incredibuild`, but the payoff — `"ib": provider == "incredibuild"` — is at line 76, inside the heredoc. The short range leaves the second half of your argument unsupported on screen.)*

> "Now the uncomfortable part, which is the actual reason I am standing here with no number.
>
> Look at this. Our runner decides whether a build was an Incredibuild build like this: `if command -v ib_console; then mode=incredibuild`. It checks whether a binary is on `PATH`. Then, twenty lines later, it writes `\"ib\": true` into the evidence file. Our `REQUIRE_IB` flag does the same thing — it checks `PATH`.
>
> So a build that executed one hundred percent locally on an unlicensed grid **would** be recorded as an Incredibuild build, and I **would** have narrated it from this stage as an Incredibuild build, and nothing in that evidence format could have told you otherwise. That is not a hypothetical about somebody else's code. That is the line you are looking at.
>
> And it is not the only one. Here is the second, and it is in the proof layer — the part I told you fails closed. These three fields — cache scope, cold-cache cleared, parent-seed cleared — are set to `true` by the tool that writes the receipt. Not measured. Asserted. The validator dutifully checks them, and they can never be false. I found that while preparing this talk.
>
> **That is the failure mode to take home.** It is not that the grid was down. It is that presence-of-tool was standing in for work-was-distributed, in two different layers, written by the same people on the same week. And the only reason I am not showing you a green slide right now is that *one* of those layers — the one that asks the coordinator for a counter instead of trusting a flag — was built the other way."

**LAND IT:**
> "We had two options. Write the instrument so it infers, and have a number. Or write it so it verifies, and risk having nothing. Where we wrote it to verify, it has nothing for us today. Where we wrote it to infer, it would have had something for us — and that something would have meant nothing. That is what a working proof gate looks like from the inside. The measurement will exist when the grid is licensed. Not before, and not on a slide."

**MANDATORY — say it plainly so nobody has to ask:**
> "So: **implemented, not measured.** Incredibuild acceleration in this system is implemented and unmeasured. I am not showing you a speedup today."

**Three things not to say in Branch B:**
- *"Our committed evidence bundles say `ib: true`."* **False.** Only 2 of 8 do, they use an old schema with no `build_provider` field, and they were not produced by the code on your screen. If you want them, say: *"two archived example bundles say `ib: true`, and they came from an earlier revision of this runner."* Better: drop the sentence. The code excerpt carries the argument alone.
- *"Every build we ran on an unlicensed grid was recorded as an Incredibuild build."* Nothing on disk establishes that any recorded `ib: true` build ran unlicensed. **Use the subjunctive**, as scripted above.
- *"I would be showing you a green slide with an entirely fictional speedup."* An inferring receipt fed local builds would produce native-vs-local medians that are roughly **equal** — a ratio near 1.000, not a speedup. Say *"a number on it that meant nothing."* This is the sentence a hostile questioner dismantles.

**And do not volunteer 21.426 / 21.948.** If asked, one sentence: *"Two historical observations, different candidates, one sample each, uncontrolled cache — and the Incredibuild one was 522 milliseconds slower. That is why they are not on a slide."*

**Fallback:** if the terminal is unavailable, B3 works entirely from three slide excerpts — the measured bail conditions, the four-line `command -v ib_console` block, and the cache-clear transcript parser that replaced the three hardcoded receipt fields. Nothing executes.

---

### BEAT 7 · 21:00–23:30 · What the evidence keeps — SHARED

**On the Linux initiator:**
```
ls -1 evidence/$RUN-b
cat evidence/$RUN-b/artifact/swf-cli.sha256
sha256sum evidence/$RUN-b/artifact/swf-cli
```
**On the Mac** (abort level L4), the last command is `shasum -a 256`. `sha256sum` is GNU-only. Name the host before you type.

> "The deliverable is not the patch. It is the patch plus evidence somebody else can inspect: the source identity, the candidate diff, the actual executable, the per-scenario results, and the full proposal-decision-dispatch trace for all seventeen episodes. The verifier recomputed that digest against the exported binary before it printed PASS. If you rebuild after exporting, it refuses.
>
> Be precise about what a digest is: **it identifies an artifact. It does not authenticate an execution history and it is not proof of general correctness.** The verdict comes from the protected checks bound to that artifact — the digest is what binds them."

**DO NOT SAY 88/88.** That describes an archived revision and an older verifier; today's verifier says seventeen scenarios. If a slide still shows it, say *"archived, older verifier, different scope"* in the same breath.

**10/10 is different and you may say it.** `tests/contract.rs` has 8 `#[test]` functions and `src/lib.rs` has 2 — exactly the "10 / 10 · 8 contract + 2 unit" the demo page prints. It still describes today's suite. Do not lump it with 88/88 on the do-not-say list.

**Never present the archived `f358e898…` digest as a verified artifact.** That executable is not committed. Any "verify the artifact" beat must use the binary exported live in 5c.

**Fallback:** if `$RUN-b` never completed, run these three commands against **`evidence/local-e2e-20260924T125134-runner-b`** and say *"rehearsal coverage"* out loud before anyone reads the run ID. That bundle is robosuite 1.5.2 / MuJoCo 3.9.0, 17 scenarios, **35 560 ms**, artifact sha256 `72e2694ebc0e8b2cb37e3993bb770e51f7d15af0ccb4755c069b411249881a45` matching manifest and sidecar, and it verifies `PROTECTED VERDICT: PASS (17 scenarios; complete coverage matrix)` offline on the Mac — re-run from disk on 2026-09-24. Second fallback: `evidence/sil-final-runner-b` (2026-09-23, robosuite, 17 scenarios, 31 992 ms), name its date. **Not** `evidence/e2e-local-20260923-194727-runner-b` — that bundle records `backend: "mock (kinematic stand-in; NOT robosuite SIL)"` and 8 976 ms of scenario time. Showing it while narrating robosuite/MuJoCo SIL is exactly the misrepresentation abort level L3 exists to prevent.

---

### BEAT 8 · 23:30–25:00 · Close — SHARED except one sentence

No command. Final slide: repository, evidence link, one scope sentence.

> "The robot lifting the cube was never enough. The candidate had to satisfy a contract it did not get to edit, survive a build in a workspace that did not exist a minute earlier, and produce evidence bound to the executable that actually made the decisions.
>
> The architecture separates temporary execution from **compilation that is meant to be reused**. Incredibuild provides the build integration. Independent checks — written in Rust, failing closed — decide whether a candidate advances."

*(Not "reusable compilation." Every `build-metrics` record in this repo literally carries `"cache_reuse_verified": false`.)*

**Branch A+ final sentence:**
> "Software-in-the-loop, segment-level authorization, no physical-robot result. What we measured is compilation wall time on one candidate, five samples per mode, with verified remote work. Everything else is a separate claim needing a separate experiment."

**Branch A− final sentence:**
> "Software-in-the-loop, segment-level authorization, no physical-robot result — and a measured result that was not the one we wanted. We built an instrument that could tell us we were wrong, and then it did. That is the part I would keep."

**Branch B final sentence:**
> "Software-in-the-loop, segment-level authorization, no physical-robot result, and no measured speedup. The gate that refused to certify our build is the same kind of gate that refused to let a cube lift on 600-millisecond-old data. **Both of them said no this week. That is the system working.**"

Memorize the last sentence of your branch. It is the line people quote.

---

### Q&A bank (5:00, outside the 25)

| Question | Answer |
|---|---|
| **Why Rust?** | A small typed decision API with deterministic clock inputs makes the contract explicit and testable. Rust does not enforce the right freshness policy for you — it makes the policy a thing you can test. |
| **Why a robot?** | Because task completion and policy compliance visibly disagree. The simulator makes that disagreement repeatable without booking hardware. |
| **What does Incredibuild accelerate?** | What it is *intended* to accelerate is eligible compilation. Not model reasoning, not simulator physics, not cached acceptance verdicts. **[Branch B / A−: "and I did not measure that today" / "and on this workload it did not."]** |
| **Why islo?** | A planned future implementation of the disposable-execution role. It has no working code in this repository today. |
| **Does the gate stop the arm?** | No. It is a pure function over protocol messages the bridge sends it. What the evidence proves is that no trace contains a dispatch the gate did not permit. A controller that moved the arm without proposing would not be caught by this crate. That is a real design limit. |
| **[A+] How do I trust it?** | Every sample's telemetry is in `evidence/<run>-build/build-proof/raw/` — raw `ib_console` output, raw Build History JSON, raw cache statistics. The verifier's conditions are in `validate_build_proof` and it fails closed. Caveat: three of the receipt's cache fields are asserted by the writer, not measured. I want to fix that. |
| **[B] Why no speedup?** | Because our verifier refuses to certify distribution from zero remote tasks. The experiment is implemented and gated; it has not produced a passing receipt. |
| **[B] Isn't that a failure?** | It is a failed measurement and a working instrument. The alternative — an instrument that inferred distribution from a binary being on `PATH` — would have given me a number today, and it would have been wrong. |
| **What about 21.4 / 21.9?** | Different candidates, one sample each, uncontrolled cache, and the Incredibuild one was 522 ms slower. They demonstrate the integration path. They are not a performance measurement and I am not presenting them as one. |
| **Were those `ib: true` runs distributed?** | We don't know. That is the point. Those two bundles come from an earlier runner revision and carry no remote-task counter. Nothing in that evidence format could answer your question, which is why we built one that can. |

---

## 4. Claim inventory — what to change, and in what order

### 4.1 Fix before the talk regardless of branch — public site

These are live on `zozo123.github.io/rust-china-conf/`, and Beat 8 sends the room there.

| Location | Current | Replace with | Why |
|---|---|---|---|
| `scripts/site/landing.html:145-146` | `<strong>21.426 <small>s</small></strong>` / `<strong>21.948 <small>s</small></strong>` — **literals**, only the *labels* are `$phase_a`/`$phase_b` | `$phase_a_value` / `$phase_b_value`, keys added to **both** language blocks of `copy.json` | **Highest-value unfixed item.** `build.py --check` exits 0 today and validates only `copy.json` key parity — it never sees these numbers. Edit `copy.json`, see green, push, and the stale pair is still live. **Do this first; it is the precondition for a minutes-long flip later.** |
| `docs/demo/loop.en.html:119`, `loop.html:105` | `cargo build --locked  (compile stage distributed)` | `(compile stage dispatched via ib_console; distribution unverified)` | Flatly asserts the one thing no committed evidence establishes. Directly contradicts Branch B's thesis. |
| `docs/demo/loop.html:101` | `空缓存命名空间` (empty cache namespace) | `远程缓存未知`, matching `loop.en.html:115` "remote cache unknown" | ZH-only over-claim — asserts exactly the controlled cache state the receipt is supposed to establish. |
| `docs/demo/loop.html:154` | `运行器 B — 新实例，热工厂` (new instance, **warm factory**) | `运行器 B — 新工作区，同一基线`, matching `loop.en.html:168` | ZH-only over-claim of a warm shared build farm. |
| `loop.en.html:113` / `loop.html:99` | frame header `— disposable, empty cache` / `— 一次性，空缓存` | drop "empty cache" / "空缓存" | The header asserts a controlled cold state in both languages while the body line right below says the cache is unknown. |
| `loop.en.html:163` / `loop.html:149` | `instance gone` / `实例已不在` | `workspace gone` / `工作区已删` | Contradicts the repo's own disclaimer in five other files (`README.en.md:116`, `README.md:101`, `plan.md:39`, `slides.*.md:101`). |
| `loop.en.html:165` / `loop.html:151` | `and the compilation work the parent revision warmed` | delete | The parent-warmed-hit claim, with no telemetry behind it. |
| `loop.en.html:193` / `loop.html:179` | `protected verifier: 88 / 88 PASS` as the **green climax frame** | remove, or relabel "historical verifier, archived revision" | Beat 7 forbids saying it; the site says it in green. |
| `docs/demo/README.md:61` | `证明加速路径运行过` (proves the **acceleration** path ran) | `证明 Incredibuild 构建路径运行过`, matching `README.en.md:85` | ZH substitutes 加速 for the neutral English "build path". |
| `docs/demo/index.html:384` | static `<dt data-i18n="evV">受保护校验器</dt>` — only the JS dicts at `:430`/`:479` add "历史 / Historical" | qualify the **static** `dt` text too | View-source, JS-disabled, saved page or a mid-load screenshot shows a bare, current-sounding 88/88 on a public site. |
| `docs/demo/index.html:387` | `<dd>10 / 10 · 8 contract + 2 unit</dd>`, no `data-i18n` | add a key so the label localizes | Cosmetic. **The number itself is correct and current** — leave it. |

**`docs/demo/loop.html` and `loop.en.html` are the most dangerous artifacts in the repo.** They are animated terminal transcripts that *show* a cold build, a warm build, an empty cache, a warm factory, distributed compilation, surviving warmed work, a destroyed instance and a green 88/88 — a complete acceleration story presented as a recorded session. The only thing between them and a false claim is a small-type bilingual note at `loop.en.html:95` / `loop.html:84`.

**After any `copy.json` edit:** `python3 scripts/site/build.py && python3 scripts/site/build.py --check`. Note that `docs/demo/index.html`, `loop.html` and `loop.en.html` are **not** generated by `build.py` and must be edited by hand.

### 4.2 The stale-number map — 21.426 / 21.948 / 522 ms

Twenty files, fifty-eight locations. The ones that matter, in flip order:

1. `scripts/site/landing.html:145-146` → `copy.json` keys (§4.1). Unblocks everything downstream.
2. `scripts/site/copy.json:99` (en) + `:207` (zh) — `measurement_note`. Once the keys exist, this single string carries the entire public measurement narrative in both languages.
3. `scripts/site/copy.json:43`+`:151` (`ib_body`, final sentence: *"A completed proof run is still required."*) and `:81`+`:189` (`value_build_body`, final sentence: *"Until that experiment produces a receipt, the speedup stays unmeasured."*). Both are trailing sentences; safe to swap without touching surrounding copy.
4. `scripts/site/copy.json:105`+`:213` (`scope_body`) — on a receipt, delete **only** the clause "and a measured performance benefit" / "及已测得的性能收益". The rest (physical robots, HIL, trained vision, continuous e-stop) stays verbatim.
5. `docs/talk/slides.en.md:130-139` and `slides.md:130-139` — the "Build observations" slide is the single projected surface built entirely on the stale pair. **A+/A−:** replace the table with native / ib-cold / ib-parent-warm medians, ranges and sample count. **B:** retitle it *"Why there is no speedup on this slide"* and replace the table with the measured bail conditions plus the live `remote_tasks: 0`. **Both decks are exactly 189 lines with matching Marp breaks — keep the counts equal.**
6. `docs/talk/talk-25min.en.md:113-135` / `talk-25min.md:99-115` — both already contain the full flip in adjacent paragraphs. On a receipt: delete the historical quote, change the last sentence of the harness paragraph. No other structural change.
7. `docs/talk/talk-25min.en.md:182-183` / `talk-25min.md:154` — on a receipt, delete the *"Why no speedup?"* Q&A bullet; replace with *"What did you measure?"* citing the run ID and the three modes.
8. `docs/talk/runbook-6min.en.md:87-92` / `runbook-6min.md:78-81` — **B: unchanged, they are already the correct stage script.** A: replace bullets 3 and 4 with the receipt's numbers and run ID.
9. `README.en.md:147-150` / `README.md:124-126` — one paragraph, both languages, same flip.

**Leave alone even on a receipt:** `docs/examples/README.en.md:41-45` and `README.md:30`. They describe the archived run, which stays uncontrolled forever. Add a forward link to the new receipt instead of editing the archive's own caveat.

### 4.3 Claims to soften regardless

| Location | Change | Reason |
|---|---|---|
| `copy.json:39`+`:147` (`stack_title`) | "Reusable compilation." / "编译成果可复用。" → "Compilation meant to be reused" | **Still soften it, for a new reason.** A parent-warmed cache-hit count now exists (47/52 and 52/52, measured 2026-09-24), so the property is no longer unevidenced — but the mechanism does not support the slogan: reuse requires the target *path* to be pinned, which makes the workspace not disposable, and keeping the directory's contents instead is 80x cheaper. See `CACHE-RESULTS.md` §3. |
| `README.md:1`, `README.en.md:1`, `copy.json:11`+`:119` | Add one sentence under the title mirroring `talk-25min.en.md:6`: the title states the motivation, not a measured conversion | "A Million Compiles. One Robot Hour." is disclaimed in the talk and slide notes but carried **bare** on both landing pages and both READMEs. **No achievable receipt would support it** — it is the one claim in the inventory the benchmark cannot fix. |
| `docs/examples/README.en.md:31-32` / `README.md:25` | "both Cargo phases executed under `ib_console`" is backed only by the runner's own `ib` flag | Weakest of the active claims, and the sentence an IB-literate audience member will probe. |

### 4.4 What flips if a receipt lands — the completeness risk

Nine files still say "no measured speedup." **Missing one means the deck contradicts itself on stage.** The order above is built so that `copy.json` (2 new keys + 4 strings) covers both public pages, and the talk / slides / runbook / README edits are four more single-paragraph changes. Everything else in the disclaimer inventory is either an archive note that should not be edited or a presenter comment only you see.

### 4.5 Disclaimers that stay true no matter what the grid does

These are not affected by a receipt and must survive every edit pass:

- **Workspaces, not machines.** `README.en.md:116`, `README.md:101`, `plan.md:39`, `slides.*.md:101`. Worktree separation is not VM or container isolation.
- **Islo is a planned provider** with zero working code — 14 tracked files. An `ISLO_SANDBOX_KEY` in `.env.local` changes nothing; `preflight.sh:51-55` says so on stdout.
- **The scripts do not invoke an LLM** — 10 locations.
- **Runners build the committed revision** (`git rev-parse HEAD^{commit}`), not your working tree.
- **A digest identifies an artifact**; it does not authenticate an execution history or establish general correctness.
- **No continuous physical safety supervision**; 250 ms is an illustrative policy, not a hardware limit; hold steps are unguarded.
- **No hardware-in-the-loop, no physical validation, no trained vision.**
- **A compilation improvement does not convert to cost or robot-hours without measuring those quantities.** *This stays true even in Branch A+* — the benchmark times `cargo build --workspace --locked` and nothing else. If the receipt lands, over-reading it into the title becomes the new exposure.

---

## 5. Bilingual status

**The brief's premise was wrong and should be struck from the footgun list.** "The ZH 25-minute script is 31 lines shorter and is missing speaker caveats" is not true. The 190-vs-159 line delta is **CJK line wrapping**: Chinese carries the same meaning in roughly 60% of the display width (5 453 display columns vs 8 752 for the same content). Structurally the two scripts are 1:1 — identical section count, identical timings and order, and paragraph blocks that align one-for-one. Every safety-critical caveat is present in Chinese: 未测量, "does not prove speedup / cache reuse / helper execution", cold/warm-are-phase-names, the 522 ms framing, the ≥5-samples gate, and digest-is-not-an-attestation.

**`wc -l` is the wrong parity metric for a CJK/Latin doc pair and will keep generating false alarms.** If any CI check or review habit compares line counts across these files, replace it with a paragraph-block comparison. **Pin the splitter before adopting it** — two independent recomputations of the per-section fingerprint disagree (`3/4/5/5/8/3/5/5/3/3` vs `3/5/6/6/9/4/6/6/4/4`) because they treat the pre-first-heading preamble differently. **The load-bearing property is EN == ZH, and that holds under every splitter tried.** Do not hardcode a published digit string as a CI fingerprint; it will never match.

### What was actually repaired (applied, verified on disk)

Eleven edits across four files in `docs/talk/`. `git diff --stat`: 4 files, +14 / −12, nothing outside `docs/talk/`.

| File | Change |
|---|---|
| `talk-25min.md:37-38` | **The only genuine factual divergence, removed.** ZH read "一段运动可以包含多个控制步，**因此**这里没有展示连续的物理安全监督" — the 因此 made multi-step segments the *reason* for the absence of continuous safety supervision. EN states three independent facts. The real reason is that authorization happens at dispatch. Now: "这里有一个重要边界：授权发生在每段运动派发时。一段运动可以包含多个控制步。我们没有演示连续的物理安全监督。250 ms 是演示用的策略阈值，不是硬件安全限。" Causal chain dropped, EN's "One important boundary" signpost restored, 开始之前 → 派发时 to match the 派发 vocabulary already at ZH:47 and :92. |
| `talk-25min.md:55` | Numeric 45-second cap added: "把单次尝试的等待上限设为约 45 秒；超时就直接切到已审阅的回退补丁". Matches `runbook-6min.md:63` exactly, so the two ZH documents stop disagreeing. |
| `talk-25min.md:101` | Both halves of the measurement-slide guard merged: "明确标注为历史数据，并标明运行编号". |
| `talk-25min.md:115` | The dropped conditional restored: "在没有实测这些量之前，不能把它换算为成本节省或机器人工时". |
| `talk-25min.en.md:44` | "…an illustrative policy, **not a hardware safety limit.**" — a caveat that existed only in ZH. |
| `talk-25min.en.md:64-65` | "…cap the attempt at approximately 45 seconds; on timeout, switch to the reviewed fallback patch." |
| `talk-25min.en.md:115` | "[Show two observations, clearly labeled as historical, **with the run ID visible**.]" — EN required only the historical label, ZH only the run ID. Each carried half the guard. Both now carry both. |
| `slides.md:58` / `slides.en.md:58` | 45-second cap appended inside the existing speaker-note comment. |
| `slides.md:165` / `slides.en.md:165` | Digest note aligned with the talk script: "…也不证明程序普遍正确" / "…and not proof of general correctness." |

### Parity now

- `python3 scripts/site/build.py --check` → **exit 0**, "Both static editions match the template, translations, and recorded matrix." **Do not read this as evidence the speaker scripts are in sync** — `build.py` reads only `copy.json` and `landing.html` and never opens `docs/talk/`.
- **Paragraph-block parity: EN == ZH** in all three file pairs.
- **Numeric-token parity: zero differences** in all three pairs.
- **Marp symmetry preserved:** `slides.md` and `slides.en.md` are both still exactly 189 lines, no slide break moved.
- **45-second cap coverage now complete** across all six stage documents. Before: two runbooks only. A presenter driving from the talk script or the deck had **no numeric stop condition on the beat most likely to stall.**

### The real bilingual risk, going forward

**It is the opposite of the one originally stated.** The ZH text was in three places *stricter or more complete* than EN. Two have now been mirrored into EN; the third (`talk-25min.md:53`'s 默认 / "by default" on the reviewed-fallback sentence) remains ZH-only. **Any future sync must be bidirectional.** If EN is treated as canonical and ZH regenerated from it, those guards are silently lost.

**Not applied, deliberately:** no `copy.json` or `landing.html` change. No talk-script caveat maps to a site copy key, and the `landing.html:145-146` finding is a claim-inventory fix (§4.1), not a bilingual-drift fix.

---

## 6. Honest limits — stated plainly

**Things this talk does not establish, in the order an audience will probe them.**

1. **Incredibuild acceleration IS measured now — and the honest limit is the opposite of the old one.** A certified cache-only receipt exists (`evidence-live/cache/receipt.json`, sha256 `da1b019d…`, schema v2, 15 samples, `BUILD RECEIPT CONSISTENT`, exit 0 under `--distribution excluded --empty-cache-hit-floor 1`). The Build Cache works: 47/52 hits after a one-file change, 52/52 on identical source, from Incredibuild's own per-task report. **It is also slower than plain `cargo` with the target directory preserved — 6,527 ms against 892 ms, 7.3x.** Every "faster than native" ratio in the receipt is measured against a native build that `ib-benchmark.sh` wipes the target directory for. Full scope in `CACHE-RESULTS.md`. The old 21.426 / 21.948 / 522 ms pair is superseded, not merely disclaimed, and the `-f` 23-second figure was `--force-remote` and must not be quoted at all.

1b. **The certification does not cover the wall times.** `wall_ms` is corroborated by nothing: a receipt with all five warm samples set to 500 ms prints `BUILD RECEIPT CONSISTENT` at `ratio=23.030x`, exit 0. Exit 0 establishes a cache state and the absence of distribution. **Never compress "certified" into "the speed-up is proven."** The 3.13x full-reuse figure is not in the receipt at all — the three-mode schema cannot carry it.

1c. **Cross-machine cache sharing is NOT demonstrated and cannot be on this grid.** `BuildCache.ServiceURL` carries no value and `BuildCacheService.SizeLimit` is 0, so the store is machine-local. **This is the only branch where the Build Cache could plausibly beat a local target directory** — a fresh container or a second machine has no target directory to preserve, and 90% reuse at a workspace path the cache has never seen would then be worth real time. It is unmeasured. Say "unmeasured", not "would".

2. **Helper execution is measured, and it is zero — in two different senses.** Under the cache-only profile rustc is declared `local_only`, so `remote_tasks=0` and `remote_core_time=0` on all 21 IB builds is the *intended* result and is what the gate demands. Separately, with `allow_remote` and **without** `-f`, this grid also distributed nothing (`numberOfRemoteTasks=0`, `maxInitiatorCores=4`) — but that is **n=1**, a smoke result, and no slide may state a distribution conclusion from it. The earlier "2.01x slower" was produced **with** `-f` (`--force-remote`), which idles the initiator's own cores; it measures a handicapped configuration, not Incredibuild.

3. **Cache reuse is measured, and the key is now understood.** The key includes the rustc **output path** (dominant) and the **CWD**, and does *not* include the user. Varying the target path alone drops reuse to 3/52; moving the workspace to a brand-new path still gives 47/52. Every earlier run's `1 hit / 52` was `fresh_target()` mktemp-ing a new `CARGO_TARGET_DIR` per sample — a guaranteed miss by construction, not a broken cache. **"Disposable workspaces, reusable compilation" survives only in a narrow form:** the cache replays 52/52 when the target *path* is held fixed and its *contents* wiped, but a path you must pin is not disposable, and if you can pin the path you can keep the contents (81 ms instead of 3,706 ms). `CARGO_TARGET_DIR` is an environment variable and need not live inside the workspace being destroyed.

3b. **The cache clear is machine-wide, and the verifier still prints otherwise.** `/opt/incredibuild/management/build_avoid_cache.sh:127` runs `rm -rf /etc/incredibuild/cache/build_cache/shared/*` unconditionally for every scope argument (measured: 102,880 KB / 105 tars → 8 KB / 0 tars from a `user clear`). `receipt-verdict.txt` line 9 still says `cache scope … local-user`. The operator gate and `method.txt` were corrected; the verifier's stdout was not. Also: `--build-cache-local-user` selects the C/C++ ccache store and is **inert for rustc**, and `/ib/mnt/fscache` (28 KB used) is the remote-execution file service, **not** the build cache.

4. **Cache discipline is transcribed, not declared (fixed 2026-09-24) — but the transcript is weaker than it looks, and it is not re-read at proof time.** The receipt no longer asserts its own cache state: cache facts are transcribed per sample from a `cache-clear.sh` transcript with a retained sha256, and an absent transcript yields *unknown* and fails closed. Three honest limits remain, all measured on 2026-09-24:
   - The transcript records what the cache tool was *asked* to do and its exit status. It does not audit the cache afterwards.
   - **It does not record the tool's name.** `cache-clear.sh` shifts `TOOL` off the argument list before writing `argv=$*`, so a transcript reads `argv=-rf /path/to/target` with no tool name and an empty body — and `rm -rf` on a nonexistent path also exits 0. **Fix this before the grid run**, because on the grid this same transcript is the only evidence `build-proof` accepts for the `ib-cold` and `ib-parent-warm` clears.
   - **`build-proof` never opens the transcript.** `check_clear_usable` (`main.rs:695-700`) validates `transcript_sha256` for shape only and never recomputes it; the file is never read at proof time. A 64-zero digest with a nonexistent path passes. See finding 3 in §"Two new findings".

4b. **The 2026-09-24 native receipt cannot distinguish its own configurations.** All 20 samples carry `mode: "native"` and an identical `source_revision`; cold/warm and `-j1`/`-j10` survive only as a substring of the free-text `build_caption`. `mode_stats` (`main.rs:619-633`) takes one median per mode and `print_build_proof` (`main.rs:977-986`) divides by `stats["native"].median_ms` — **3 938 ms, a bimodal midpoint matching no build anyone ran.** The four per-config receipts alongside it (5 homogeneous samples each) are the honest containers. **Never let the 20-sample median become the baseline a future grid receipt is divided into.**

5. **The safety gate is advisory over self-reported protocol messages.** `decide` is a pure function. `session.rs:333` catches a violation only when the bridge *reports* `dispatched: true` for a non-permitted action. The invariant proven is "no trace in this evidence pack contains a dispatch the gate did not permit," not "the arm cannot move on stale perception." Hold steps are explicitly unguarded and never reach the gate at all.

6. **The receipt would not measure the build the audience watches.** `ib-benchmark.sh` times `cargo build --workspace --locked`. The runner builds `cargo build --locked` plus `cargo test -p robot-safety-gate -p swf-app --locked --no-run`. Different workloads. Say so, or the first person who diffs the two scripts says it for you.

7. **The robotics beat's own builds cannot be correlated to any counter, ever.** `runner-common.sh:62-63` invokes `ib_console cargo build --locked` with **no `-c <caption>`**, no `-f`, no cache flags, no `--workspace`. `parse_ib_history` (`main.rs:218`) matches records by caption. So `build-history` can only ever describe the separate hand-typed `ib_console` command in A2/B2 — never the `cold.sh`/`warm.sh` builds on screen. **Beat 5a's `build provider: incredibuild` is a `PATH` inference with no counter behind it and no way to get one.**

8. **No physical hardware, no hardware-in-the-loop, no trained vision, no learned policy.** Cube state comes from the simulator. 250 ms is an illustrative policy value.

9. **The workspace is disposable; the machine is not.** Git worktrees and fresh target directories, removed on exit. Not VM isolation. No EC2 provisioning or destruction.

10. **No coding agent is wired in.** `scripts/robot-demo/` invokes no LLM anywhere. The rehearsal applies a reviewed patch.

11. **Islo has no working code.** Named as a planned provider in 14 tracked files.

12. **The title is not a result.** "A Million Compiles. One Robot Hour." is a motivation. **No receipt this pipeline can produce would support it** — the benchmark measures compilation wall time and licenses no claim about cost, CPU-hours or robot-hours. If the receipt lands, over-reading it into the title becomes the new exposure.

---

## 7. What to fix in the repo, when there is time

Not required for the talk, but each of these is a real defect the talk currently works *around*.

| File | Change | Priority |
|---|---|---|
| `scripts/robot-demo/preflight.sh:25-32` | Hard-FAIL when `ROBOT_DEMO_BACKEND=robosuite` and the import fails. Today it prints `warn` and exits 0, so preflight is **not a gate** for the SIL path Beat 5 depends on. | High |
| `scripts/robot-demo/preflight.sh:41-49` | When `IB_HISTORY_URL` and `IB_CLIENT_API_KEY` are set, run one captioned `ib_console` build and assert `remote_tasks > 0`. **This is the T-30 branch decision, automated**, and the only check that distinguishes a licensed grid from a silent local build. | High |
| ~~`rust/crates/swf-cli/src/main.rs:826-828`~~ | **DONE 2026-09-24.** Cache scope and both clearing flags now come from a parsed `cache-clear.sh` transcript (sha256 retained); absent transcript = *unknown* and fails closed; v1 receipts rejected; two regression tests guard it. | ~~High~~ Closed |
| `scripts/robot-demo/runner-common.sh:57,76` | When `REQUIRE_IB=1`, verify remote execution occurred before writing `"ib": true`, rather than inferring from `command -v ib_console`. | High |
| `scripts/robot-demo/runner-common.sh:62-63` | Pass `-c <caption>` to `ib_console` so the runner's own builds become correlatable to Build History at all. | High |
| `scripts/robot-demo/ec2-agentic-physical-ai.sh:37-47` | Add `SKIP_BUILD_EXPERIMENT` / `SKIP_BEHAVIOR`, or run the phases with recorded status, so a benchmark failure cannot swallow the behavior rehearsal under `set -e`. | Medium |
| `scripts/robot-demo/ec2-agentic-physical-ai.sh:16-17` | Also require `ROBOT_DEMO_PYTHON` (absolute, executable) and `IB_ALLOW_CLEAR_USER_CACHE=1`. Today the wrapper exports `REQUIRE_IB` and `ROBOT_DEMO_BACKEND` but not these two, so the robosuite bridge silently falls back to system `python3`. | Medium |
| `scripts/robot-demo/ib-benchmark.sh:22,48` | Move the `swf-cli` bootstrap **above** the env gates, and honor or explicitly override `CARGO_TARGET_DIR` when locating `PROOF_CLI`. | Medium |
| `scripts/robot-demo/*.sh` | Stop sourcing `.env.local` with `set -a` *after* the caller's exports, or at minimum print what it overrode. | Medium |
| `README.md:55`, `README.en.md:65` | The `validate.sh one-run --scenario fresh_lift` example needs the exact-set constraint stated inline (`verify_run.py:207`). **This string is in the READMEs, not the runbooks** — the runbooks' `validate.sh stage-b` example is already correct. | Low |
| `docs/talk/runbook-6min.en.md:32-42` and `runbook-6min.md` | The visible-sequence table is anchored to 0:00–6:00; the new Beat 5 is 9:00–15:30 with a parallel-pane `warm.sh` start at 10:45. Re-anchor and add the pane-2 start as its own row. | Low |
| `docs/talk/slides.*.md` | Add one Branch-B slide after the measurement slide: the four-line `command -v ib_console` excerpt plus "presence of a tool was standing in for work was distributed". Add it **symmetrically** to both decks. | Low |
