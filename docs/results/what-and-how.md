# WHAT TO DO, HOW TO DO IT, AND WHAT IS MISSING

**As of 2026-09-24.** Repo `/Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf`, branch `main`, HEAD **`a3c35f9`** — unchanged. Working tree: **18 modified, 2 untracked**. Nothing committed, nothing pushed, no PR, no ssh. `git stash list` empty.

The grid is unreachable (VPN down; route to `10.133.0.0/16` leaves via `en0`, no tunnel). Everything below marked **[local]** can be done on the Mac today. Everything marked **[grid]** cannot.

**The one-line state of the world:** the repo is now honest, and the code is now fixed — but **no *validated* benchmark receipt has ever existed**, so *no acceleration claim is defensible*, and two files the entire stage path depends on are **not in git**.

> **Updated 2026-09-24 (later the same day) — see `DEMO-RESULTS.md`.** A receipt now exists: `evidence/build-exp-20260924T100157/build-proof/receipt.json`, schema v2, 20 samples, sha256 `a42d40f3…10ff5d2`. It is **native-only, produced on this Mac with no Incredibuild present**, and `build-proof` **rejects** it (`Error: missing benchmark mode ib-cold`, exit 1). So "no validated receipt" and "no defensible acceleration claim" both still hold — but the flat sentence "no receipt has ever existed" is no longer true, and the same day's work found that `build-proof` **accepts a hand-forged receipt** (`ratio=11.948x`, exit 0). Read `DEMO-RESULTS.md` §5.2 before quoting anything in this file about what the verifier checks.

---

## LEDGER 1 — DONE (no grid needed)

Every row was re-verified on disk today, not taken from an agent summary.

| # | What is now fixed | File(s) | Command that proves it |
|---|---|---|---|
| 1.1 | **P0 cleared.** `swf-cli robot-demo build-proof` exists in the built binary, along with three subcommands nobody had catalogued: `build-history`, `build-sample`, `build-receipt`. The "stale binary dated Sep 23 16:42" premise was itself stale. | `rust/target/debug/swf-cli` | `cd rust && cargo build && target/debug/swf-cli robot-demo --help` → 7 subcommands, build is a no-op |
| 1.2 | **The tautology is gone, not moved.** `cache_scope`, `cache_cleared_before_each_cold_sample`, `cache_cleared_before_each_parent_seed` are deleted from the struct. Schema is **v2** (`main.rs:155`). Cache state is now per-sample, transcribed from `cache-clear.sh`, and **fails closed when absent**. v1 receipts are rejected by name. | `rust/crates/swf-cli/src/main.rs` | `grep -c cache_cleared_before_each rust/crates/swf-cli/src/main.rs` → `0`; `grep -n 'BUILD_PROOF_SCHEMA_VERSION: u32' main.rs` → `155: = 2` |
| 1.3 | **A regression guard that fails if anyone re-hardcodes it.** `cache_state_is_never_asserted_by_construction` reads `include_str!("main.rs")` and asserts the `build-receipt` dispatch arm sets no cache field itself. It has been shown to fire. **This is the single best thing you can put on screen.** | same | `cd rust && cargo test -p swf-cli` → **17/17 pass** |
| 1.4 | **The verifier stopped overclaiming in its own output.** `BUILD PROOF PASS` → **`BUILD RECEIPT CONSISTENT`** (`main.rs:968`), plus an explicit CHECKED-FROM-RECORDS list and a **NOT CHECKED** paragraph saying it cannot detect a fabricated receipt. | same | run `build-proof` against any receipt; read the last paragraph |
| 1.5 | **`.env.local` no longer silently overrides the operator.** New shared loader gives the caller's environment precedence and announces ignored values on stderr. Wired into `preflight.sh:7`, `runner-common.sh:10`, `rehearse.sh:14`, `ib-benchmark.sh:13`. | `scripts/robot-demo/env-local.sh` **(UNTRACKED)** + 4 callers | `bash -n scripts/robot-demo/*.sh`; `shellcheck -x scripts/robot-demo/*.sh` → exit 0 |
| 1.6 | **The `evidence/<RUN_ID>-build` path confusion is gone.** `ec2-agentic-physical-ai.sh:15-21` derives `BUILD_RUN_ID` / `BUILD_PROOF` **once** and gates every stage through `require_path()`, which on absence prints the expected path, the producing script, and an `ls` of the parent. | `ec2-agentic-physical-ai.sh`, `ib-benchmark.sh` | `sed -n '15,40p' scripts/robot-demo/ec2-agentic-physical-ai.sh` |
| 1.7 | **`CARGO_TARGET_DIR` can no longer misdirect the bootstrap build.** `ib-benchmark.sh` names its own target dir and **refuses to run — before clearing any cache —** if the freshly built CLI lacks `build-proof`. | `ib-benchmark.sh:~68` | `sed -n '60,75p' scripts/robot-demo/ib-benchmark.sh` |
| 1.8 | **The landing page's two headline numbers are now data-driven and self-qualifying.** `21.426` / `21.948` moved out of `landing.html` into `copy.json` (111 keys per edition) with new `$phase_a_flag` / `$phase_b_flag` warn slots: EN *"522 ms SLOWER than A · no speedup established"*, ZH *"比 A 慢 522 ms · 未确立任何加速"*. Plain text, no JS required. | `scripts/site/landing.html`, `copy.json`, `docs/index.html`, `docs/en/index.html` | edit `21.426`→`99.999` in `docs/index.html`, then `python3 scripts/site/build.py --check` → **exit 1**. (Verified. Before this change the gate was blind to that edit.) |
| 1.9 | **`88 / 88` now carries its "historical revision, not the current verifier" qualifier in RAW HTML**, so a view-source / JS-off / screenshot reader sees it. Both i18n dicts updated to match. `class="pass"` removed so a stale count no longer renders green. | `docs/demo/index.html:384` | strip all `<script>` blocks, grep `88 / 88` → qualifier present inline |
| 1.10 | **`10 / 10` kept (it is TRUE) and finally given `data-i18n="evTT"`**, so the number/unit text translates instead of being English-only in both editions. | `docs/demo/index.html:387` | `grep -n 'evTT' docs/demo/index.html` |
| 1.11 | **ZH brought down to the weaker EN everywhere it over-claimed.** `空缓存命名空间`→`远端缓存状态未知`; `热工厂`→`新 worktree，同一基线`; README's `证明加速路径运行过`→`不是加速测量`. English was never strengthened. Untelemetried assertions now read *"illustrative: distribution unverified"* and *"instance teardown unverified"*. | `docs/demo/loop.html`, `loop.en.html`, `docs/demo/README.md` | `git diff docs/demo/` |
| 1.12 | **The repo's real gate is green.** `scripts/robot-demo/check.py` PASS (15 stages, incl. site-check, expected-seed-contract-failures, patched-matrix, patched-protected-verifier). `cargo build --locked` exit 0, `Cargo.lock` unmodified, clippy `-D warnings` clean. | — | `python3 scripts/robot-demo/check.py` |

**Correction to the numbers you were handed:** `cargo test --workspace --no-fail-fast` is **33 passed / 3 failed**, not "7 passed / 3 failed" (that was the `robot-safety-gate` crate alone). On the **patched** revision it is **36/36** (2 unit + 8 contract + 9 session_protocol + 17 swf-cli), not 27/27 — `swf-cli` grew from 8 to 17 tests. The 3 failures are the seeded conference fixture (`robot-safety-gate/src/lib.rs:147` is literally `let _ = (age_ms, policy); Decision::Permit`). **That failure is the demo. Do not fix it.** But the runbook must say so out loud, because anyone in the audience who clones `main` and runs `cargo test` sees red.

---

## LEDGER 2 — WHAT ONLY YOU CAN UNBLOCK

Ranked by how much each one blocks. Rows 2.1–2.5 are the grid. **Row 2.0 is not — and it is the most urgent thing on this page.**

### 2.0 · `git add` the two untracked files — **[local], 30 seconds, blocks everything**

**What:** `scripts/robot-demo/env-local.sh` and `scripts/robot-demo/cache-clear.sh` are **not in git**. Four stage-path scripts hard-require them.

**How:**
```bash
cd /Users/yossi.eliaz/Documents/Codex/2026-09-23/thi/rust-china-conf
git add scripts/robot-demo/env-local.sh scripts/robot-demo/cache-clear.sh
# then commit them WITH the 18 modified files, in one commit. (I did not commit — not authorised.)
```

**Unblocks:** any clone, any `git clean -fd`, any `git stash -u`, any `git archive`, and any rsync-of-tracked-files to the EC2 runners.

**Cost if it never happens:** a tracked-files-only checkout is **dead on arrival**. Materialised and run:
```
scripts/robot-demo/preflight.sh: line 7: scripts/robot-demo/env-local.sh: No such file or directory
```
Immediate abort under `set -euo pipefail`. The entire demo path dies at line 7 of the first script.

**And the gate cannot save you.** `check.py:24` copies with `git ls-files --cached --others --exclude-standard` — it *deliberately includes untracked files*. So `check.py` is green while a real clone is broken. **Today's green `check.py` is NOT evidence that the stage path is shippable.**

### 2.1 · GlobalProtect VPN — **[you], blocks the entire measurement track**

**What:** reconnect the corporate VPN. Grid hosts are `10.133.20.216`, `10.133.27.32`, `10.133.29.30`, `10.133.10.80`.

**How:** connect GlobalProtect, then `nc -z -G 4 <INITIATOR_IP> 22 && echo VPN-PASS`.

**Unblocks:** every `[grid]` row below, and the only path to a receipt.

**Cost if it never happens:** Branch B permanently. No acceleration number, ever. You can still show the code, the guard test, and the robotics beat — those are ~6:30 of live demonstration that do not touch the grid.

### 2.2 · An Incredibuild licence valid for **this** coordinator — **[you/operators], blocks the receipt even with VPN up**

**What:** confirm the coordinator can load its licence keypair and that **helper agents report non-zero licensed cores** on the machine you will actually use. The licence on disk is named `Yossi-ec2-test_` and may be bound elsewhere. Alternative: get sanctioned use of the SaaS broker token.

**How (on the Initiator, once VPN is up):**
```bash
cd /tmp && ib_info > /tmp/ib_info.$$.txt 2>&1
grep -iE 'helper|licen|core' /tmp/ib_info.$$.txt
```
PASS = every helper listed with a **non-zero** licensed core count. FAIL = any `no available or licensed cores on helper machine`, or zero helper cores.

**Unblocks:** `remote_tasks > 0`, which is a hard condition in `validate_build_proof`. Without it every sample fails.

**Cost if it never happens:** **no receipt, ever** — this is the known blocker B2. Worse, a build that executes entirely locally is still *recorded* as an Incredibuild build, and nothing in the evidence format tells you otherwise. Use the subjunctive on stage; do not publicly diagnose your employer's product ("our operators are still working the cause").

### 2.3 · Real `IB_HISTORY_URL` and `IB_CLIENT_API_KEY` — **[you], blocks the receipt**

**What:** the Build History endpoint and client API key. `ib-benchmark.sh:49-52` exits 2 without both.

**How:** export them on the Initiator, or (new behaviour as of today) put them in `.env.local` — the new loader makes operator exports win, so this can no longer silently override you.
```bash
export IB_HISTORY_URL=... IB_CLIENT_API_KEY=...
export IB_ALLOW_CLEAR_USER_CACHE=1   # also required; clears only qa_user's local IB cache
export IB_SAMPLES=5                  # minimum enforced; build-proof --min-samples defaults to 5
```

**Unblocks:** the unique-caption → Build History match, `remote_tasks`, `remote_core_time_s`. Five of the verifier's eight structural checks read from this.

**Cost if it never happens:** benchmark exits 2 before touching anything. No receipt.

### 2.4 · Outbound egress from the grid hosts to `static.rust-lang.org` and PyPI — **[you/network], blocks the run mid-flight**

**What:** the grid hosts are VPN-only. `ib-benchmark.sh` runs `cargo fetch` and `rehearse.sh` needs Python deps.

**How, before the measured run:**
```bash
cd "$REPO/rust" && cargo fetch --locked && echo FETCH-PASS
```

**Unblocks:** keeps every download out of the measured intervals.

**Cost if it never happens:** the benchmark **dies before any measurement**, and it dies late — after you have already cleared caches. Do this first, not during.

### 2.5 · An exclusive grid window — **[you], blocks the *validity* of the numbers**

**What:** a block of time where nobody else's builds are on the coordinator or helpers.

**How:** book it with the grid operators; confirm at the start with `ib_info`.

**Unblocks:** medians that mean something.

**Cost if it never happens:** contention noise on a 20-build experiment. And note the verifier's limit: its "no other cache-using build in between" check only sees builds **recorded in this receipt**. Another user on the shared Initiator is invisible to it. An exclusive window is the only thing that closes that hole.

---

## LEDGER 3 — WHAT IS STILL MISSING, AND WHY IT MATTERS

### 3.1 · No *validated* receipt has ever existed. No acceleration claim is currently defensible. **[the big one]**

> **Superseded in part on 2026-09-24 — see `DEMO-RESULTS.md`.** `find evidence -name receipt.json` now returns one file: the native-only `build-exp-20260924T100157` receipt, which `build-proof` **rejects**. Nothing below about *Incredibuild* changes: 0 IB builds, 0 acceleration measurements, 0 acceleration claims.

At the time this section was written, `find evidence -name receipt.json` → **nothing**. `grep -rl build-proof docs/ scripts/site/` → **nothing**. The entire proof layer — schema v2, `verify_cache_chain`, 17 tests, the regression guard — **has never produced an artifact and is invisible to the public site and the talk.**

The `21.426` / `21.948` on the landing page are the *old uncontrolled A/B*, not build-proof output. They are one sample per phase, caches uncontrolled, and **Incredibuild was 522 ms SLOWER**.

**So: the only measurement you have says Incredibuild lost.** That is now correctly disclosed on the site. It is also the honest possibility you must be prepared for: *once measured properly, Incredibuild may still be slower on this workload.* Cargo already parallelises well; a 20-crate Rust workspace is not a 10,000-file C++ build. Decide **before** stage whether Branch B's thesis survives that answer. (It does — Branch B is about evidence discipline, not about the number — but only if you have not pre-committed to a speedup in the first three minutes.)

### 3.2 · The consistency checker is not tamper-evident, and I built a passing fake in two minutes

A hand-written `receipt.json` with 15 samples, every `transcript_path=/nonexistent/never-written.txt`, every `transcript_sha256` = 64 `a` characters, no history file, no cache tool, no Incredibuild, no clear ever performed, printed:
```
BUILD RECEIPT CONSISTENT  run=fabricated-by-hand   ...ratio=3.000x   exit 0
```
The tool discloses this in its NOT CHECKED paragraph, so it is not dishonest. But three cheap fixes close most of it, and the evidence is already on disk:

- **The digest is decorative.** `check_clear_usable` (`main.rs:674-711`) only checks that `transcript_sha256` is 64 hex chars and the path is non-empty. **Nothing ever opens the file.** *Fix:* in `verify_cache_chain`, re-read `transcript_path`, recompute with `evidence::sha256_file`, re-run `parse_cache_clear`, assert equality; add `--evidence-root` so the path must live inside the run's evidence dir; add a test that mutates one byte and expects a bail.
- **Scope is trusted, not re-derived.** `scope` and `argv` round-trip as two independent JSON fields and only `scope` is read. Setting `argv="all clear --everything-including-shared"` while leaving `scope="local-user"` still passes. *Fix:* factor the argv→`CacheScope` match out and re-derive at validate time, bailing on disagreement. **Two lines.**
- **A false doc comment, which is itself a new unearned assertion.** `main.rs:428-431` says `parse_cache_clear` "is the only way a `CacheClear` can come into existence". It is not — `CacheClear` derives `Deserialize` and `build-receipt` reads `BuildSample` straight from hand-written JSONL. *Fix:* reword to "the only way `build-sample` creates one."
- **`print_build_proof` overclaims twice** (`main.rs:998-1006`): "one Build History record per caption reporting `success`" cannot be checked by `build-proof` at all (no status field on `BuildSample`; the match happens upstream in `build-sample`), and "no other cache-using build between" is true only of builds *in this receipt*. *Fix:* "checked against this receipt's own records" / "no other build **recorded in this receipt**". The same overclaim is in a comment at `ib-benchmark.sh:292-294` ("checks the receipt against the retained transcripts" — it does not).

### 3.3 · The runbook's own preflight gate P6 is now broken by the schema-v2 change

`PREFLIGHT-AND-RUNBOOK.md:160-162` tells the operator to run `build-sample --mode ib-cold --repetition 1 --wall-ms 1 --caption ... --history ... --cache ...`. Run verbatim against today's binary:
```
error: the following required arguments were not provided: --started-at-ms <STARTED_AT_MS>
exit 2
```
On the Initiator this reads as **CACHE-PARSE-FAIL and aborts the run for the wrong reason**. The corrected P6 needs `--started-at-ms "$(date +%s%3N)"` **and** `--cache-clear <transcript>` (now mandatory for every IB mode). `ib-benchmark.sh`'s own `cache_stats()` at `:139-142` is the working template. **Fix this before the VPN comes up, not during.**

### 3.4 · The two authoritative planning docs are now stale in the *opposite* direction — reading from them on stage states a falsehood about your own repo

- `FINAL-TALK-E2E.md:28` — "the verifier fails closed with no inference anywhere → PARTLY FALSE"
- `:46`, `:330`, `:501`, `:624`, `:652` — all still say `main.rs:826-828` hardcodes the three fields and scripts the line *"I would like to fix that before I show you this again."*
- `PREFLIGHT-AND-RUNBOOK.md:431`, `:500`, `:570` — same.

**All of that is now false.** The literals are gone, the schema is 2, v1 receipts are rejected by name, and there is a test that fails if anyone puts them back.

Also: the phrase *"fails closed with no inference anywhere"* **never appeared in the repo**. Grepping `docs/talk/` for it and for 推断 returns zero hits. It exists only in these two planning docs. No repo file needs that fix — only these two do.

**The replacement sentence, exact:** *"No inference — and no assertion either. Every cache fact is transcribed from the operation that produced it, and anything unobserved fails closed."* And retire *"the verifier proves the acceleration path ran"* — the tool now prints `BUILD RECEIPT CONSISTENT` and says in its own output that it is a consistency check over a self-reported receipt, not a proof.

**Branch B got stronger, and you have to decide which version you tell.** It is no longer "I found a defect I have not fixed." It is **"I found it, I fixed it, and the fix is a test that fails if anyone re-hardcodes it"** — and you can put that guard failing on screen in about four seconds. That is a better story. It is also a different story from the one scripted in both docs.

### 3.5 · Public claims still standing, unfixed

| Where | What it still says | Why it matters |
|---|---|---|
| `docs/demo/loop.html:99` / `loop.en.html:113` | `phase:"运行器 A — 一次性，空缓存"` / `"Runner A — disposable, empty cache"` — this renders into `<h1 id="phase">`, **the largest text on the page** | The dim sub-line one line below was softened to "remote cache unknown". The headline still asserts an empty cache. **The over-claim survived the softening, in both editions, in the most prominent element.** |
| `docs/examples/README.en.md:17,38` and `README.md:13,28` | "protected verifier **88/88 PASS**" | Today's verifier prints `PROTECTED VERDICT: PASS (17 scenarios)`. Line 3 of each file carries a blanket archive banner, but lines 13/17/28/38 read as current. `docs/talk/*` already carries qualifiers; `docs/examples` is the last place 88/88 reads as today's number. |
| `docs/demo/index.html` | prints a sha256 for an executable **not committed to the repo** | The landing page's provenance section tells the reader a clone cannot recheck it. The demo page does not. |
| `docs/demo/loop.*.html` | `ib_console: coordinator + initiator + 2 helpers` | Zero supporting telemetry; covered only by the page-top blanket note, not inline like the two assertions that were fixed. |

### 3.6 · Gaps in the gates themselves

- **`docs/demo/*` is not covered by any automated check.** `build.py` writes only `docs/index.html` and `docs/en/index.html`; `.github/workflows/validate.yml` runs only `build.py --check`. The 88/88, the 10/10, the sha256 and both loop pages can silently drift again. A raw-HTML-vs-i18n-dict consistency check for `docs/demo/index.html` is the obvious missing piece and does not exist.
- **`build.py --check` enforces key-set equality, not value equality** (`build.py:46` is `set(copy["zh"]) != set(copy["en"])`). Verified live: setting `en.phase_a_value` to `77.777` while `zh` stays `21.426`, rebuilding, still exits 0. **~3 lines** to add a numeric-value-equality assertion.
- **`check.py` includes untracked files** (see 2.0). Structurally blind to the one regression introduced this session.

### 3.7 · Small, sharp, and cheap

- **`ts_ms()` is unguarded on macOS.** `cache-clear.sh:21-30` carefully guards `date +%s%3N` with a python3 fallback; `ib-benchmark.sh:106` is a bare `ts_ms() { date +%s%3N; }` — and it now feeds the newly-*required* `--started-at-ms`. On this Mac it yields `17902398493N`, which clap rejects. Linux-only in practice, but the two functions should share one `now_ms`.
- **The regression guard's anchor is fragile.** `main.rs:1747` locates the dispatch arm with the literal `"RobotDemoCommands::BuildReceipt {\n                samples,"` — leading whitespace and all. One `rustfmt` run and the `.find` silently misses, changing what the guard inspects. The guard is the strongest artifact you have; anchor it on something that does not encode indentation.
- **Caption uniqueness is not enforced across samples.** `validate_build_proof` dedups repetitions but not `build_caption`, so two samples may cite the same Incredibuild build. One dedup check.
- **Revisions remain shell claims.** `candidate_revision`, `parent_revision` and each `source_revision` are asserted by the shell (`--source-revision "$CANDIDATE_ID"`), never observed. The new `candidate != parent` check helps. "This build was that revision" is still not evidence — worth saying on stage.
- **Schema v1 receipts are rejected outright, not migrated.** Any receipt in `evidence/` from a previous run now fails `build-proof`. If you had one you intended to show, it must be regenerated — and that needs the grid.

---

## THE MOMENT THE VPN COMES UP

Exact order. Each step has a gate. **Do not proceed past a red gate** — every one of these failures is cheaper before the measurement than during it.

**Do this on the Mac first, while still disconnected:**

0. **`git add` both untracked files and commit the 20-file change set.** (Ledger 2.0.) Nothing reaches the runners until this is done — a tracked-files-only copy dies at `preflight.sh` line 7.
1. **Fix runbook P6** (§3.3): add `--started-at-ms "$(date +%s%3N)"` and `--cache-clear <transcript>`. Copy the shape from `ib-benchmark.sh:139-142`.
2. **Fix `ts_ms()` in `ib-benchmark.sh`** (§3.7) — one line, and it is on the critical path of every sample.

**Then, tunnel connected:**

3. **VPN reachability.** `nc -z -G 4 <INITIATOR_IP> 22 && echo VPN-PASS`. Red → stop, reconnect. Persistent → abort level L4, Branch B.
4. **SSH.** `ssh -o ConnectTimeout=5 qa_user@<INITIATOR_IP> true`.
5. **Sync the repo to the Initiator** — *from the commit in step 0*, not from an rsync of tracked files against an uncommitted tree.
6. **Licence gate.** `ib_info | grep -iE 'helper|licen|core'`. Every helper must show **non-zero licensed cores**. Red → **stop. No receipt is possible today.** Go Branch B and do not spend the window.
7. **Egress gate.** `cd "$REPO/rust" && cargo fetch --locked && echo FETCH-PASS`. Red → fix egress before anything measured; the benchmark dies late otherwise.
8. **`.env.local` gate (P7).** `cat "$REPO/.env.local"` — it should set `ISLO_SANDBOX_KEY` and nothing else. With the new loader your exports win regardless, but you still want to *know*.
9. **Build the CLI on the Initiator and confirm `build-proof` exists.** `ib-benchmark.sh` now refuses to run without it **before** clearing any cache — but confirm it here so the refusal is not your first surprise.
10. **Parser gate (corrected P6).** `build-history --build-number-only` → `show_build_cache_statistics.sh` → `build-sample` with `--started-at-ms` and `--cache-clear`. Red with `expected one unambiguous cache hits counter` → **this IB version's format does not match `parse_cache_counters` (`main.rs:263`). Do not attempt a live receipt.**
11. **Export the run environment.**
    ```bash
    export IB_HISTORY_URL=... IB_CLIENT_API_KEY=...
    export IB_ALLOW_CLEAR_USER_CACHE=1 IB_SAMPLES=5
    unset CARGO_TARGET_DIR      # a stale export kills the run ~20-30 s in
    ```
12. **Burn one throwaway run.** The full IB path has never executed with the new `cache-clear.sh` wrapper, the new required flags, or `IB_STATS_TOOL` parsing on the *seed* build — which is now on the critical path where it was not before. **Expect the first run to fail. Budget for it.** Use a disposable `RUN_ID`.
13. **The real run, inside your exclusive window.** 20 full `cargo build --workspace --locked` runs plus up to 24 s of Build History polling per IB sample — **10-15 minutes**. Captions must be globally unique; never re-run a curl with a different caption to get a nicer answer.
14. **Verify the receipt.**
    ```bash
    "$REPO/rust/target/debug/swf-cli" robot-demo build-proof \
      --receipt "$REPO/evidence/<RUN_ID>-build/build-proof/receipt.json" --min-samples 5
    ```
    Expect `BUILD RECEIPT CONSISTENT` plus the NOT CHECKED paragraph. `remote_tasks: 0` → that build ran locally; say so and do not claim it.
15. **Copy the receipt and its `raw/` directory back to the Mac**, so Beat 6 runs offline from the copy. The receipt is a **pre-talk artifact**. Never run `ib-benchmark.sh` or `ec2-agentic-physical-ai.sh` on stage — the latter chains benchmark→rehearsal under `set -euo pipefail`, so a benchmark failure at repetition 3 of 5 would kill the robotics beat entirely.
16. **Then, and only then, update the site and the talk with the measured number** — whatever it says, including if it says Incredibuild is slower.

---

## SCOPE AND HARD-RULE AUDIT

HEAD is still `a3c35f9`; `git reflog -3` shows no commit after it; `git stash list` is empty; no push, no PR, no ssh. The 4 pre-existing `docs/talk/` modifications are intact and untouched (all four are caveat additions). Nothing under `docs/assets/`, `docs/examples/`, `Cargo.lock` or `robot-safety-gate` was modified. `rust/target/debug/swf-cli` was rebuilt — gitignored build output, no `rust/` source change implied by it. Leftover gitignored evidence dirs (`evidence/verify-112329-runner-a`, `-runner-b`, `evidence/check-*`) can be deleted freely.
