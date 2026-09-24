# CACHE-RESULTS — Incredibuild Build Cache on a 52-unit Rust workspace

**Measured 2026-09-24 on grid AL6555, initiator `AL6555_I_1_0` / `10.133.27.234` (4 cores, 16 GB).**
Repo `~/rust-china-conf` @ `f721855259e857efc008f1e84e270c92263c876d`, Rust 1.92.0, dev profile.
Every Incredibuild counter in this document comes from Incredibuild's own per-task Build Cache
report (`--build-cache-report-all-miss`) and Build History, cross-checked against the build
report DB. **No cache state anywhere in this document is inferred from wall time.**

Evidence: `evidence-live/cache/` (receipt `da1b019d…`, verdict transcript, two run bundles:
`cache-only-20260924T122940Z` 92 files + `cache-extras-20260924T123437Z` 38 files, ~3.1 MiB
together) and `evidence-live/cache/native-warm-control-20260924T130233Z/` (the control run this
document turns on, scripts and raw timings retained).

---

## 1. TL;DR

The workflow set out to measure Incredibuild's Build Cache in isolation from distribution.
**It succeeded: the cache was isolated, it demonstrably works, and it is slower than doing
nothing special with cargo.**

> ### The one honest headline number
>
> ## 892 ms
>
> **Median wall time of one agentic-loop iteration — apply a one-file candidate patch to a
> 45-dependency, 52-unit Rust workspace and rebuild — using plain `cargo` with the target
> directory preserved.** n=5, range 883–931 ms, measured today on the initiator, same patch and
> same `cargo build --workspace --locked` the benchmark times.
>
> The same iteration costs **6,527 ms** through Incredibuild's Build Cache (n=5, 47 of 52
> compilations served from cache, `remote_tasks=0`), and **11,515 ms** from an empty target
> directory (n=5).

So: the loop is **12.9x** faster than a from-scratch build, and Incredibuild's warm Build Cache
is **7.3x slower** than simply keeping the target directory.

**Why this is the number and not `3.13x faster` or the certified `1.76x faster`.** Both of those
are real measurements against a native baseline that `ib-benchmark.sh` wipes before every single
sample (`native_sample() → disposable_workspace()`). Incredibuild was handed a persistent store
that survives across samples and is deliberately seeded with the parent revision; native was
denied any persistent store at all. Three independent reviewers landed on that objection, and the
control that settles it had never been run. It has now been run (§2, bottom block). It reverses
the sign of the result.

**The publishable finding is a negative one, and it is a good one:** on a Rust workspace whose
crate graph is deep rather than wide, Incredibuild's Build Cache cannot beat cargo's own target
directory, because cargo's incremental store already solves the same problem locally and solves
it two orders of magnitude faster on the units that hit. That is measured, it is reproducible,
and it survives the first question from the audience — which `3.13x` does not.

**One thing Incredibuild is strictly better at, and it is not speed** — see §6.

---

## 2. The modes in one table

All medians are of n=5 with rotating run order, one machine, idle (load 0.08–1.28), `cargo fetch
--locked` outside every timed phase, `cargo build --workspace --locked` in every row.

### 2a. What the gated run measured (`evidence-live/cache/`)

| # | Mode | Median | Range | n | IB cache hits | remote tasks | Gated |
|---|---|---:|---|---:|---|---:|---|
| A | **native**, target dir wiped first | 11,515 ms | 11,458–11,549 | 5 | — | — | yes |
| E | **IB cache-only, COLD cache**, target wiped | 16,049 ms | 15,456–16,176 | 5 | **1 / 52** | **0** | yes |
| G | **IB cache-only, WARM**, one-file change, target wiped | 6,527 ms | 6,449–6,626 | 5 | **47 / 52** | **0** | yes |
| F | **IB cache-only, WARM**, identical source, target wiped | 3,706 ms | 3,378–4,310 | 5 | **52 / 52** | **0** | no |
| A′ | native anchor (phase 2 re-measure) | 11,608 ms | 11,473–11,804 | 5 | — | — | no |
| A2a | native, `debug=0` | 10,399 ms | 10,328–10,508 | 5 | — | — | no |
| A2b | native, explicit `-fuse-ld=lld` | 11,619 ms | 11,490–11,694 | 5 | — | — | no |

The two native anchors agree to 0.8% and the explicit-lld run agrees with the default to 0.1%;
that pair is the run's own precision estimate. Counters were identical in all five repetitions of
every IB mode. `remote_tasks = 0` and `remote_core_time = 0` on **all 21** Incredibuild builds
across both phases, from Build History — the cache is genuinely isolated from distribution.

### 2b. The control that was missing, measured today

`evidence-live/cache/native-warm-control-20260924T130233Z/` — same machine, same revision, same
patch (`demo/fallback-patch.diff`, the one `ib-benchmark.sh` applies), same timed command.

| # | Mode | Median | Range | n |
|---|---|---:|---|---:|
| N0 | native, persistent target, **nothing changed** (no-op rebuild) | **81 ms** | 77–81 | 5 |
| N1 | native, persistent target, **one-file patch applied in place** | **892 ms** | 883–931 | 5 |
| N1′ | native, persistent target, same patch **reverted** in place | 908 ms | 884–965 | 5 |
| N2 | native, **brand-new worktree path per candidate**, one shared persistent target | **2,778 ms** | 2,775–2,867 | 5 |
| — | native, empty target (control's own cold seed) | 11,565 ms | — | 1 |

The cold seed lands within 0.4% of the receipt's native median, so the control and the gated run
are measuring the same machine in the same state.

### 2c. The comparison, stated once

| Comparison | Result |
|---|---|
| IB warm one-file (6,527) **vs** native persistent target, patch in place (892) | **IB 7.3x SLOWER** |
| IB warm one-file (6,527) **vs** native fresh worktree + shared target (2,778) | **IB 2.35x SLOWER** |
| IB full reuse (3,706) **vs** native no-op rebuild (81) | **IB 46x SLOWER** |
| IB cold (16,049) **vs** native from scratch (11,515) | **IB 39% SLOWER** (0.72x, +4,534 ms) |
| IB warm one-file (6,527) vs native **from scratch** (11,515) | IB 1.76x faster ← *the certified number, handicapped baseline* |
| IB full reuse (3,706) vs native **from scratch** (11,608) | IB 3.13x faster ← *same defect, and never gated* |
| native, patch in place (892) vs native from scratch (11,515) | **cargo 12.9x faster than itself** |

**It is the same work on both sides.** cargo's output on the N1 rebuild reads
`Compiling robot-safety-gate / swf-app / swf-cli` — *exactly* the three crates that MISS in
Incredibuild's 47/52 warm build. Both configurations recompile three crates. Native serves the
other 49 units by checking they are already on disk (81 ms); Incredibuild serves 47 of them by
unpacking cache tars (~5.6 s). **Cache replay is two orders of magnitude slower than cargo's
freshness check.** This is not an unfair native advantage; it is the mechanism.

There is no branch on this grid where the cache wins on time:

- keep the target directory → native wins by 7.3x;
- discard the workspace, relocate the target directory → native wins by 2.35x;
- move to a fresh machine → the IB cache is empty and starts at 16,049 ms against 11,515 ms;
- share the cache between machines → **not possible here**, see §7.

---

## 3. Root cause: why every earlier run showed 1 hit out of 52

Every one of the previous run's 15 builds reported `ib_cache_total=52, ib_cache_hits=1`. That was
not a broken cache. It was the benchmark asking for a guaranteed miss.

**The Build Cache key includes the rustc output path.** `ib-benchmark.sh`'s `fresh_target()`
`mktemp`s a *new* `CARGO_TARGET_DIR` for every sample, and every real compile carries that
absolute path in `--out-dir`, `-L dependency=` and `--extern`. ~0% reuse **by construction**.

What the key actually contains, measured one factor at a time against a warm cache (21 controlled
builds, ids 35–55):

| Factor varied | Result (52 compilation units) |
|---|---|
| nothing (control) | **52 hits / 0 miss** |
| `CARGO_TARGET_DIR` path | 3 hits / 49 miss ← **dominant** |
| workspace source directory (byte-identical content) | 47 hits / 5 miss |
| user identity (`qa_user` → `root`, uid 0) | 52 hits / 0 miss ← **not in the key** |
| one source file changed, both paths fixed | 49 hits / 3 miss |

- **Target path is in the key.** The 3 survivors are cargo's probe invocations (`rustc -vV`,
  `rustc - --print=file-names …`) whose command lines contain no path.
- **CWD is also in the key**, which was not previously known. The identical command line
  `rustc -vV` hashes to `3a69c800…` under one workspace and `18937b72…` under another. That
  costs exactly the 3 workspace crates + 2 cargo probes = 5 misses when the workspace moves; all
  45 registry dependencies have CWD under `~/.cargo/registry/` and hit regardless. A workspace at
  a brand-new path still gets **47/52 = 90% reuse**.
- **User is not in the key.** A `root` build was served 52/52 from entries `qa_user` stored.
- Toolchain was not varied (only 1.92.0 installed). The rustc absolute path is in the command, so
  a toolchain change is certainly a key change — claim no more than that.

### "Disposable workspaces. Reusable compilation." — what actually survives

The measurement is real: **hold the target path stable, wipe its contents, and 52 of 52
compilations are served from cache**, reproduced after the 09:56 reboot (builds 82–87), so the
pre-reboot `iso-fixed-D` result is re-established. The pillar is no longer a slogan.

**But the mechanism does not support the slogan.** The cache only replays if the "disposable"
workspace's *target directory* lands at the same absolute path every time. A path you must pin is
not disposable — and if you can pin the path you can keep the contents, which is 81 ms instead of
3,706 ms. Worse, `CARGO_TARGET_DIR` is an environment variable: it does not have to live inside
the workspace you are destroying. Point a genuinely fresh worktree at one shared persistent target
and cargo does the candidate in 2,778 ms against Incredibuild's 6,527 ms (§2b, N2).

**The honest, narrower claim the talk may keep:** the Build Cache buys reuse where the target
directory genuinely *cannot* be preserved — a fresh container, a different user, a different
checkout — and there it delivers 90% reuse at a workspace path it has never seen. On this grid
that reuse is still slower in wall time than preserving the directory, so it is a portability
property, not a speed property.

---

## 4. Distribution vs cache, stated plainly

**Distribution is out of the story, and the reason is structural, not a vendor failing.**

- The previous run's "Incredibuild is 2.01x slower" (23,173 ms vs 11,518 ms native) was measured
  with `-f`, which is `--force-remote` ("force allow_remote tasks to remote helpers",
  `ib_console --help`; `ib-benchmark.sh:176`). Every remotable task was *forced* onto two m5.large
  helpers while the initiator's own 4 cores sat idle — Build History recorded
  `maxInitiatorCores=0`. That is the wall time of a deliberately handicapped configuration: 4
  remote cores across a network, shipping multi-megabyte rlibs, *replacing* 4 local cores. **It
  is not a fair measurement of Incredibuild and must not be quoted as one.**
- Re-run with `allow_remote` and **without** `-f`, this grid distributed *nothing*:
  `numberOfRemoteTasks=0`, `maxInitiatorCores=4`, wall times indistinguishable from cache-only
  (15,712 vs 15,928 cold; 6,575 vs 6,509 warm). "Distributed" without `-f` **is** cache-only in
  disguise on this workload. ⚠️ **This rests on n=1** (`evidence/dist-smoke-152151`); it is a
  smoke result, and no slide may state a distribution conclusion from it.
- Why this is expected: rustc is **one process per crate**, and this crate graph is deep rather
  than wide (45 packages, ~52 units). Cargo pipelining already overlaps much of the critical path
  locally. Proc-macro crates (`serde_derive`, `thiserror_impl`) and build scripts must be built
  *and run* on the host, so they cannot be distributed and they sit on the critical path. There is
  simply not much width to sell to helpers.
- Under the cache-only profile there is nothing left to distribute anyway: wall times were
  identical to `allow_remote` in every cache state.

**Cache-only is directly supported and was the primary configuration here.** `rust/ib_profile.xml`
with `type="local_only"` plus `<ib_cache enabled="true"/>` keeps rustc intercepted and cached
while nothing is ever sent to a helper. `local_only` is a legal `type_type` value in
`/opt/incredibuild/data/ib_profile.xsd`, whose enumeration is exactly five values —
`intercepted`, `static_intercepted`, `static_intercepted_fileops`, `allow_remote`, `local_only`.
(An earlier report listed a sixth, `low`; that value belongs to `local_slot_priority_type`. The
benchmark's own `profile-check.txt` reads the correct five, so nothing downstream inherited the
error.) The reviewable profile is `rust/ib_profile.cache-only.xml`, sha256
`20d976e763a43fa623a52d41e798f1d621de5f8a6d66df0c6513a4cff94cb02d`, differing from the shipped
profile in **exactly one attribute**. `xmllint` is not installed on this grid, so the script does a
targeted enumeration check against the XSD rather than full schema validation, and says so.

### Rust-native levers, measured rather than assumed

| Lever | Result |
|---|---|
| `debug=0` (dev profile currently `debuginfo=2`) | 10,399 ms vs 11,608 ms = **1.12x**. Real but small, and it costs you debuggers. |
| Faster linker (lld/mold) | **Not available — already taken.** rustc 1.92.0 links with LLD 21.1.3 by default on this target. Explicit `-fuse-ld=lld` produced a byte-identical `.comment` section and the same wall time (11,619 vs 11,608 ms). Verified with `readelf`, transcript retained. |
| Preserve `CARGO_TARGET_DIR` | **12.9x** (892 ms vs 11,515 ms). By an order of magnitude the largest lever on this repo, and it is free. |

---

## 5. Certification — what the gate says, and exactly what it covers

**Yes, a certified cache receipt exists.**
`swf-cli robot-demo build-proof` returns **`BUILD RECEIPT CONSISTENT`, exit 0** over
`evidence-live/cache/receipt.json`, sha256
`da1b019daeaa895a1818d32154ce29280d4cf9acdbb8d165b64c78c3527062ce`, schema v2, 15 samples.

The literal default command **refuses** (`Error: ib-cold sample 1 has no verified remote tasks`,
exit 1). Certification needed two flags the *verifier* supplies and the receipt cannot set, and
each was confirmed independently necessary:

```
swf-cli robot-demo build-proof --receipt <path> --min-samples 5 \
        --distribution excluded --empty-cache-hit-floor 1
```

**Neither flag is a weakened threshold, and that was verified rather than asserted.**

- `--distribution excluded` is a **demand in the opposite direction**: every IB sample *and*
  every parent seed must report `remote_tasks == 0` and `remote_core_time == 0`, with an absent
  counter treated as *unknown*, not zero. Flipping one sample to `remote_tasks=1` was refused. The
  default `required` is simply the wrong contract for a profile declaring rustc `local_only`.
- `--empty-cache-hit-floor 1` is a **corrected contract**. The old rule (`hits == 0`) is
  factually unsatisfiable for Rust: cargo invokes `rustc -vV` twice per build; the first MISSes
  and stores, the second HITs the entry the first just wrote, with the hit's execution timestamp
  inside that same build's window. Confirmed independently from
  `/etc/incredibuild/log/2026-Sep-24/local-67`, `-71`, `-79` (hash `0d8cdb76…`). The floor is
  symmetric — it also forces warm builds to *exceed* it, and lowering warm hits 47→1 was refused.
  It defaults to 0 in `main.rs:1060`, it is verifier-supplied, and a test pins that. **Caveat: the
  symmetry is weak in practice — a warm build need only report ≥2 hits. It rules out the
  degenerate "warm == cold" case and nothing more. Do not describe it on stage as a strong bar.**
- If a literal 0/0 reference is ever wanted: `<ib_cache enabled="false"/>` measured `total=0
  hits=0 misses=0` with the store unchanged at 8 KB / 0 tars.

The gate still bites: 5 of 6 forgeries refused (leaked remote task, digest altered by one
character, nonexistent transcript path, warm hits at the floor, one newline appended to a
transcript). The briefing's warning that build-proof accepts a hand-forged receipt is now **out of
date for invented evidence** — that exact forgery is pinned dead by a test named
`the_receipt_that_printed_ratio_11_948x_from_invented_fields_is_now_refused`. 42/42 tests pass.

### What "certified" does **not** cover — read this before quoting exit 0

1. **Not the wall times.** `wall_ms` is corroborated by nothing. Setting all five warm samples to
   500 ms still prints `BUILD RECEIPT CONSISTENT`, `ratio=23.030x`, exit 0. Since every headline
   ratio is arithmetic over `wall_ms`, the certification establishes **cache state and the absence
   of distribution, and nothing about the speed numbers.** The tool discloses this itself under
   `NOT CHECKED`. No slide may compress "certified" into "the speed-up is proven."
2. **Not the 3.13x.** The receipt schema carries only `native` / `ib-cold` / `ib-parent-warm`, so
   the full-reuse mode (3,706 ms, 52/52) was measured but never gated. The certified cache
   speed-up is 1.76x — and §2c shows what that 1.76x is measured against.
3. **Not the counters.** `build-proof` re-reads only the 10 cache-clear transcripts (opened,
   re-hashed, re-parsed; all matched). It re-reads no Build History response, no cache-statistics
   output, no per-task Build Cache report and no parent-seed counter. The counters *are* true — I
   re-derived them with `grep -c '^HIT:'` over the live IB logs `local-67..87` and every one
   matched — but that confirmation is external to the gate.
4. **Not the store's emptiness.** The clear transcripts record tool, argv, `exit_code=0` and two
   timestamps with an empty body. Coldness rests entirely on the 1/52 counter. A `du` and file
   count of the store before and after each clear would close this cheaply and was not done.
5. **The receipt prints one false line.** `receipt-verdict.txt` line 9 says *"cache scope
   re-derived from 10 corroborated clear transcript file(s): local-user."* The measured blast
   radius is **machine-wide**: `/opt/incredibuild/management/build_avoid_cache.sh:127` runs
   `rm -rf /etc/incredibuild/cache/build_cache/shared/*` unconditionally for *every* scope
   argument (measured: 102,880 KB / 105 tars → 8 KB / 0 tars from a `user clear`). The operator
   gate and `method.txt` were corrected; the verifier's own stdout was not.
6. **Nothing binds the receipt to a commit.** The certified `swf-cli` was built from a dirty
   working tree (`M rust/Cargo.lock`, `M rust/crates/swf-cli/src/main.rs`,
   `M scripts/robot-demo/ib-benchmark.sh`, `?? rust/ib_profile.cache-only.xml`, …). Also: `swf-cli`
   is **not** on PATH on the initiator, contrary to the briefing; it must be built first.

### Where the cache actually lives — two corrections to the briefing

- The rustc `ib_cache` store is **`/etc/incredibuild/cache/build_cache/shared`** (one `.tar` +
  one `-manifest.json` per cached execution). Empty 8 KB / 0 tars → 47,152 KB / 51 tars after one
  cold build. Verified again while writing this: 94,280 KB / 83 entries.
- `/etc/incredibuild/cache/build_avoid/<user>.<uid>` is the **C/C++ ccache** store. It stayed at
  140 KB and 0.0 GiB across all 21 builds and never held a Rust entry. **`--build-cache-local-user`
  selects this directory and is therefore inert for rustc**, despite appearing in every argv.
- **`/ib/mnt/fscache` is not the build cache.** It is the loopback ext4 for the remote-execution
  file service, backed by the preallocated 10 GB image `ib_cache.storage`. It measured 28 KB used
  through every build (`df`, verified again today). The "11G" in the original briefing was the
  preallocated image file. Report cache size with `df`/`du` on the store, never on the image.

---

## 6. What the speaker may say on stage

Every sentence below is true as written. Say them in this order.

1. *"We set out to measure Incredibuild's build cache on a Rust workspace, with distribution
   switched off at the profile — one word changed, `allow_remote` to `local_only`. Twenty-one
   builds, every one reporting zero remote tasks."*
2. *"The cache works. Forty-seven of fifty-two compilations served from cache after a one-file
   change; fifty-two of fifty-two when the source is unchanged. Those are Incredibuild's own
   counters, not our stopwatch."*
3. *"And it is slower than doing nothing. Six and a half seconds, against eight hundred and
   ninety-two milliseconds for plain cargo with the target directory left alone."*
4. *"Both do the same work. cargo says `Compiling robot-safety-gate, swf-app, swf-cli` — exactly
   the three crates that miss in Incredibuild's warm build. The difference is the other
   forty-nine: cargo checks they are on disk in eighty-one milliseconds; the cache unpacks
   forty-seven tarballs in five and a half seconds."*
5. *"An empty cache is worse than no cache: sixteen seconds against eleven and a half. That is
   the price of writing fifty-one entries, and it belongs on the slide next to everything else."*
6. *"Distribution is not the story either, and not because the tool is bad. rustc is one process
   per crate, and this graph is deep, not wide. Forty-five packages, fifty-two units, proc-macros
   and build scripts pinned to the host on the critical path. There is very little width to sell."*
7. *"The one thing the Build Cache is strictly better at is not speed — it is not being wrong."*
   Then show the repro: share one target directory across two worktrees, give the changed file an
   older mtime, and **cargo returns in 83 ms having compiled nothing, shipping an rlib
   byte-identical to the parent's** (`0f0c4b82e21d5aec…` both times) — a stale artifact, reported
   as success. After `touch`, 2,797 ms and a different rlib. cargo's fingerprint is mtime-based;
   Incredibuild's key is command-line/content-based and cannot fail this way. **For a talk whose
   subject is a safety gate deciding whether a robot may move, "the fast path cannot silently
   serve you yesterday's binary" is a stronger argument than any ratio.**
8. *"What we can prove is a cache state and the absence of distribution. The gate does not check
   our wall times — it says so itself — so treat every ratio on these slides as a measurement, not
   a proof."*

**Do not say:** "3.13x faster", "1.76x faster", or "certified speed-up" without stating in the
same breath that the native baseline rebuilt from an empty target directory and that the
preserved-directory baseline is 892 ms. **Do not say** "disposable workspaces, reusable
compilation" without §3's mechanism caveat. **Do not say** anything about distribution helping or
hurting from the without-`-f` run: it is n=1. **Do not quote** the earlier `-f` numbers (23,173 /
22,297 ms) as a measurement of Incredibuild at all. **Do not quote** the mechanic's n=2/n=4
figures (`3780 / 3915 ms`, `native n=2`, `one-file patch n=2`); they violate the ≥5 rule and are
superseded by the n=5 medians here — the pooled n=9 median of 3,918 ms disagrees with the n=5
median of 3,706 ms by 5.7%, well outside this run's 0.1–0.8% precision.

---

## 7. Honest limits

1. **Cross-machine cache sharing is NOT demonstrated, and cannot be on this grid.**
   `BuildCache.ServiceURL` carries no value in `/etc/incredibuild/db/incredibuildConfiguration.db`
   (verified today) and `BuildCacheService.SizeLimit` is 0, so the store is **machine-local**. The
   whole measurement is one machine reusing its own entries. **What it would take:** stand up the
   build-cache service on a reachable host, set `BuildCache.ServiceURL` to
   `http://<host>:<port>`, restart the agents, size `BuildCacheService.SizeLimit` above 0, and
   pass `--build-cache-service[=<URL>]`. **This is the only branch where the Build Cache could
   plausibly beat a local target directory** — a second machine, or a fresh CI container, has no
   target directory to preserve, and 90% reuse at a path the cache has never seen (§3) would then
   be worth real time. It is unmeasured. Say "unmeasured", not "would".
2. **One machine, one workspace, one toolchain.** 4 cores, 16 GB, Rust 1.92.0, 45 packages, ~52
   units, dev profile with `debuginfo=2`. A wider crate graph, a C++-heavy build, or a bigger grid
   could invert every conclusion here. Nothing in this document generalises past this repo.
3. **The control run in §2b is mine and is not gated.** n=5 per mode, tight ranges (883–931,
   2,775–2,867, 77–81), effect sizes of 2.35x–46x, on the same idle machine within three minutes
   of the gated run. Sample count is not what carries the conclusion — but these numbers have not
   been through `build-proof`, and a receipt schema that cannot express "native with a warm target
   directory" is itself a finding worth fixing.
4. **A reviewer figure corrected, in Incredibuild's favour.** An earlier review reported 636–645 ms
   for the fresh-worktree + shared-target configuration. That loop recycled the *same* worktree
   path, so cargo's incremental state for that path survived. At a genuinely new path each
   iteration the honest number is **2,778 ms** (n=5), which is what §2 uses throughout.
5. **Output equivalence passed, but by luck of the run.** IB-cold, IB 52/52-cache-replayed and
   plain cargo at the same target path all produced a byte-identical `swf-cli`
   (`efff0853e7709ead…`) and `cargo test --workspace` passed on the cache-replayed tree. **Nothing
   in `ib-benchmark.sh` checks this** — no artifact hashing, no test step. It should assert it.
6. **`IB_SMOKE=1` results are not measurements.** The single-sample rehearsal lowers the sample
   floor to 1 and then refuses to call `build-receipt` or `build-proof` at all, dropping a
   `SMOKE-NOT-A-MEASUREMENT.txt` marker. Nothing downstream may quote it.
7. **The clear is machine-wide.** Anyone else using this box loses their entire rustc Build Cache
   every time this benchmark runs a cold sample. That is now stated in the operator gate and in
   `method.txt`; it was previously documented as "clears only qa_user's local IB cache", which is
   false.
