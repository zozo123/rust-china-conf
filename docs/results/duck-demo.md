# The duck demo — does it work, and would I put it on stage?

**Substrate:** microduck (pollen-robotics, Apache-2.0), 23 workspace members, 576 locked
packages, 117k lines of Rust, MuJoCo body, 50 Hz control loop.
**Scratch clone:** `/private/tmp/claude-502/-Users-yossi-eliaz-Documents-Codex-2026-09-23-thi-outputs/f19722cb-85eb-4861-8238-90950fc19489/scratchpad/microduck` at `e4ae26f`, 3 local commits on
top of upstream `a9ec4b2`. Never pushed, no PR, working tree clean.

Two complete runs exist and every number below is labelled with which one it came from.

| | **Run A** (bundled) | **Run B** (independent repro) |
|---|---|---|
| id | `20260924T134454Z` | `WRITEUP-REPRO` |
| bundle | `outputs/evidence-live/duck/20260924T134454Z/` | `<scratchpad>/repro-run/` |
| wall | 3 m 18 s (198 s) | **3 m 14 s (193.89 s), exit 0** |
| seeded binary sha256 | `0f2ad89c…` | `173da4cc…` |
| repaired binary sha256 | `593c4def…` | `a9111010…` |
| seeded manifest commit | `7512443`, `git_dirty: true` | `e4ae26f`, **`git_dirty: false`** |
| seeded verdict | `FAIL (seeded; 2 of 53 checks)` | `FAIL (seeded; 2 of 53 checks)` |
| repaired verdict | `PASS (repaired; 57 checks; complete coverage matrix)` | `PASS (repaired; 57 checks; complete coverage matrix)` |

---

## 1. TL;DR

**Yes. It works end to end, in one command, on this Mac, and I reproduced it independently.**

    $ DUCK_DEMO_RUN_ID=WRITEUP-REPRO DUCK_DEMO_OUT=<scratchpad>/repro-run \
        /usr/bin/time -p scripts/duck-safety-demo
    ...
    == the whole arc took 3 m 14 s
    real 193.89
    EXIT=0

One command shows the seeded revision's three named tests red, stands a duck up in MuJoCo,
sweeps for where this duck's balance actually gives out, drives seven scenarios against a
sha256-bound `robotd`, has a protected verifier **require** that run to fail on exactly the two
scenarios past the bound, offers a bounded patch to an allowlist of one file, rebuilds, drives
the same seven, and requires the second run to pass. It wrote a 65 MB evidence bundle and put
the duck back on its feet.

This matters because two adversarial reviewers ran the same command eight times between them
and got eight exit-1s. Both disclosed the confound and neither would attribute the failures:
they were running the demo *against each other* — same fixed body port 7801, same
`~/.cache/duck-sim` state directory, one of them reverting the other's applied patch mid-build.
I killed every stray `robotd`/`body_server`, confirmed the machine was quiet, and ran it once.
It passed first attempt. **The arc is real; the harness has no mutual exclusion, and that is a
staging problem, not a correctness problem.** Section 7 names all five fragilities.

### The single most striking moment

Not the one the demo we ported from has. **The duck goes down, and every instrument says it is
fine.**

    scenario  stale_100ms_turn   --stale-sensors-ms 100        [Run B, seeded]

    trunk height   0.1040 m  ->  0.0521 m        (standing is ~0.116 m)
    robotctl health at that exact moment:
        robot     healthy
        loop      49.8 of 50.0 Hz · 530 ticks · 1 missed
        bus       ok
        imu       ready
    move.limited_by on all 84 driving ticks:   {}          <- empty
    observation_age_us, median:                109.7 ms    <- the bound is 80 ms

The age was on the wire the whole time. Nothing read it. `let _ = age;` — one line, at
`duck-control/src/safety.rs:423`, in a function whose doc comment above it states the rule in
full.

Then the patch, and the same scenario:

    [Run B, repaired]   held.  stale_observation on 89 of 89 driving ticks.
                        referee displacement 0.0003 m.

**Be honest about what "held" means here** — see §7.4. The repaired duck at 100 ms does not
hold a *stand*. It holds where it was put, on the floor, because the injector is a start-time
flag and the gate correctly refuses the very first dispatch. It never rises. That is the rule
working, and it is not the nicer sentence.

---

## 2. The contract

    observation_max_age = period × (COAST_TICKS + 1) = 20 ms × 4 = 80 ms
                                                     = 4 control periods

Every term is already in microduck, and none of them was chosen for a slide.

| term | value | where it already lives |
|---|---|---|
| `period` | 20 ms | `robotd/src/main.rs:1101`, `params.period()`; loop measured 48.7–50.4 of 50.0 Hz across both runs |
| one period unavoidable | +1 | `main.rs:1997` — *"the observation is already a tick old by construction: at 50 Hz the policy is trained on data of exactly this age"* |
| `COAST_TICKS` | 3 | `main.rs:137` — *"Three ticks (60 ms) covers a drop, a retry and a slow tick; past that the robot genuinely cannot see, and holding still is the honest answer."* |
| the bound | **80 ms** | `robotd-params/src/lib.rs:1783`, `observation_max_age_ticks: 4`; registered at `robotd-params/src/registry.rs:258` |

The old harness's 250 ms was, in its own words, *"an illustrative policy, not a hardware
limit."* This one is not illustrative. It is the oldest sample the existing design already
tolerates: past four periods `Coast::sample` has already given up, so the gate and the coast
meet exactly and 80 ms is where they touch.

**The sentence to read aloud:**

> "Eighty milliseconds is not a number I picked. It is four control periods — the coast's own
> three, plus the one the policy was trained to expect. The loop reports its own missed ticks,
> so this is arithmetic, not policy. And the shipped revision throws it away."

---

## 3. The arc, step by step, with real output

Everything below is verbatim from **Run B** unless marked. Commands run in the microduck
checkout with `DUCK_SIM_RL`, `HEADLESS=1`, `DUCK_SIM_VIEWER=0`, `PATH="$HOME/.local/bin:$PATH"`.

### (a) The seed — exactly three tests, named in advance

    $ cargo test -p duck-control --lib
    FAILED  a_stale_observation_holds_the_pose
    FAILED  a_tick_with_no_observation_is_refused
    FAILED  the_freshness_bound_is_the_configured_one
    test result: FAILED. 80 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out

The harness asserts the *set*, not the count — "three tests failed" is satisfied by any three,
and one unrelated red test means the tree is broken rather than seeded. Then it prints the line
and its number:

    423
            let _ = age;

Everything a reviewer would inspect says the rule exists: `Limit::Stale`, the wire name
`stale_observation` in `move.limited_by`, `SafetyConfig.observation_max_age`,
`safety.observation_max_age_ticks` in the params registry, `RobotState.observation_age_us` on
the IPC wire. **Only the comparison is absent, and the comparison is the one thing no artifact
shows.**

### (b) A duck stands up

    == standing up
      loop      50.4 of 50.0 Hz · 322 ticks · 2 missed · last 15 ms ago
      bus       ok
    == duck-a: standing (trunk 0.116 m, gravity z -1.000)

### (c) Where this duck's balance actually gives out — measured every run

The design asks for the largest injected delay at which the dispatch age clears 80 ms and the
duck *still walks*, so the hook can be "the task succeeded, the contract was never evaluated" —
and says to report the absence rather than fabricate it. The harness measures it every run.

    -- injected  60 ms -> age   65.9 ms  fell   travelled 0.512 m  trunk_z 0.0359
    -- injected  80 ms -> age   88.6 ms  fell   travelled 0.297 m  trunk_z 0.047
    -- calibration: no

**Answer: no.** The balance cliff sits *below* the bound the coast implies. Run A measured the
same answer with different numbers (60 ms → 63.8 ms, 80 ms → 83.7 ms, both fell), and a
reviewer independently measured 65.6 and 86.6 ms. One sample per row; the rows move by up to
20 ms between runs (§7.5).

### (d) The seeded matrix — seven scenarios, 58.0 s

    -- robotd restart, --stale-sensors-ms 0
    -- fresh_walk        -> walked   age_p50=0.5ms    limits={}
    -- fresh_turn        -> turned   age_p50=0.5ms    limits={'deadman': 13}
    -- deadman_stop      -> walked   age_p50=0.5ms    limits={'deadman': 53}
    -- robotd restart, --stale-sensors-ms 40
    -- borderline_40ms   -> walked   age_p50=53.6ms   limits={}
    -- robotd restart, --stale-sensors-ms 100
    -- stale_100ms       -> fell     age_p50=108.9ms  limits={'joint_range': 21}
    -- stale_100ms_turn  -> fell     age_p50=109.7ms  limits={'deadman': 12, 'joint_range': 21}
    -- robotd restart, --stale-sensors-ms 0
    -- body_gone         -> stopped  age_p50=0.4ms    limits={}
    -- matrix (seeded) took 58.0 s

Note what `limited_by` carries under 100 ms staleness: `joint_range`. The gate that *did* fire
is the joint clamp, because the duck was thrashing into its own actuator limits on the way
down. `stale_observation` appears zero times.

### (e) The protected verifier, required to fail

      - stale_100ms: the contract requires every dispatch to be refused at a median age of
        108.9 ms, past the 80 ms bound — but only 0 of 87 driving ticks carried stale_observation
      - stale_100ms_turn: ... 109.7 ms ... 0 of 84 driving ticks
    PROTECTED VERDICT: FAIL (seeded; 2 of 53 checks)
    and it failed on exactly the two scenarios past the bound, which is the seed working

Two things make this a verifier rather than a description, and both were earned the hard way —
the harness's first verifier *passed* the seeded run, having graded the bug as correct
behaviour:

1. `refusal_required = median_age > max_age_ms` is derived **per scenario** from the manifest's
   bound and the trace's own ages. One rule for both revisions. Nothing anywhere records what a
   given revision is expected to do.
2. The harness requires the seeded run to fail **on exactly `stale_100ms` and
   `stale_100ms_turn`** and dies otherwise. "The seeded run failed" is satisfied by a flaky
   scenario or a duck that fell for its own reasons.

Expectations come from `scripts/duck-demo/scenarios/*.json` in the verifier's own checkout,
never from the evidence directory. Numbers come from the JSONL traces, never from
`scenario-results.json` — which is cross-checked against them.

### (f) The bounded repair

    $ python3 scripts/duck-demo/check-patch.py demo/freshness-patch.diff
    candidate allowlist passed: duck-control/src/safety.rs
    applied: +22 -1  duck-control/src/safety.rs

    $ cargo test -p duck-control --lib
    test result: ok. 83 passed; 0 failed

The patch replaces `let _ = age;` with the comparison, the `None => true` arm, the run counter,
the `tracing::warn!`, `applied.limits.push(Limit::Stale)` and the hold-write. A reviewer
independently confirmed the allowlist refuses an out-of-scope file: a patch touching
`robotd/src/main.rs` gets `PATCH REJECTED: candidate may only change duck-control/src/safety.rs`,
exit 1.

### (g) The repaired matrix, and the verdict flip

    -- borderline_40ms   -> walked   age_p50=57.2ms   limits={}
    -- stale_100ms       -> held     age_p50=109.7ms  limits={'stale_observation': 85}
    -- stale_100ms_turn  -> held     age_p50=112.7ms  limits={'deadman': 14, 'stale_observation': 89}
    -- matrix (repaired) took 58.2 s

    PROTECTED VERDICT: PASS (repaired; 57 checks; complete coverage matrix)

The verdict flips on exactly two of seven scenarios. The other five are identical in both
halves, which is what makes the diff legible: nothing was traded for the fix.

    scenario           inject   age p50    seeded                  repaired           [Run B]
    fresh_walk            0ms     0.5ms    walked   0/84 stale     walked   0/92 stale
    fresh_turn            0ms     0.5ms    turned   0/105 stale    turned   0/97 stale
    deadman_stop          0ms     0.5ms    walked   0/113 stale    walked   0/116 stale
    borderline_40ms      40ms    53.6ms    walked   0/90 stale     walked   0/90 stale
    stale_100ms         100ms   108.9ms    FELL     0/87 stale     held    85/85 stale
    stale_100ms_turn    100ms   109.7ms    FELL     0/84 stale     held    89/89 stale
    body_gone             0ms     0.4ms    stopped  0/37 stale     stopped  0/39 stale

### (h) The bundle

65 MB, of which ~58 MB is the two sha256-bound debug `robotd` binaries. Per-scenario JSONL from
`robotctl monitor --json` (one line per tick at 50 Hz), a referee trace of the body's own trunk
pose read straight from MuJoCo, `robotctl health` verbatim as read at each scenario, both
manifests, both verdicts, both test logs, the calibration sweep, and a README that says in as
many words what the run does *not* show.

The bundle re-verifies independently: a reviewer ran `verify.py` from the microduck checkout
against `outputs/evidence-live/duck/20260924T134454Z` and got seeded FAIL (`--expect-fail`
rc=0) and repaired PASS (rc=0). It also resists tampering — appending one byte to
`artifact/robotd` produces *"the exported executable's sha256 does not match the manifest"*, and
rewriting a trace to claim 0.4 ms ages is caught twice, by the results/trace cross-check and by
the scenario's declared `median_age_ms` range.

---

## 4. Structural versus checked

The thesis: *a check is weaker than a structure.* microduck already argues this in its own
comments — `Safety` owns the only `RobotIo` handle, so "nothing commands a motor except through
here" is, in its words, *"a fact the borrow checker enforces rather than a convention."*

### What the port made unrepresentable

**1. A backend cannot date a sample it did not take.** `Sensors.observed_at` is a **private
`std::time::Instant`**. `Instant::now()` is its only constructor; there is no way in safe Rust
to make one in the past. A backend can keep a stamp, copy it, buffer it, hand it over late — it
cannot *write* one. `Sensors::observed(..)` is the only way in from outside the module.

> This is the strongest thing in the port and it is checkable: `..Sensors::default()` stopped
> compiling in `safety.rs`'s own tests. The rule broke code the moment it existed.

**2. "I checked the age" stopped being a claim a caller can make.** `ObservationAge` has no
`Default`, no `ZERO`, no `From<Duration>`, no public field. `ObservationAge::of(&Sensors)` is
the only constructor — the type asks for the sample, every time.

> Consequence in the tests: the stale-observation test cannot fake an age. `aged(by)` genuinely
> `std::thread::sleep`s, because there is no other way to obtain an old one.

**3. A dispatch site that forgot about freshness does not compile.** `Safety::apply` takes
`age: Option<ObservationAge>` by value and by type, as a fourth parameter. Adding a new caller
without an observation is a compile error, not a review finding.

**4. `None` is a refusal, not a young age.** The one case a rule written as a comparison gets
backwards is "I was not told." Losing the sensor entirely must not default-authorize.

**5. The wire name could not be forgotten.** `limit_name`'s match over `Limit` is exhaustive, so
the compiler forced `Limit::Stale → "stale_observation"` to exist on the wire in the *seeded*
revision. That is why every artifact a reviewer inspects says the rule is there.

### What is only checked, and the hole in the thesis

**The freshness comparison itself ships as a runtime `if` inside `apply`.** That is precisely
the shape the thesis calls weaker. The design's intended finale — a `Fresh<'a>` witness type
that makes a stale dispatch unrepresentable rather than detected, `demo/structural-patch.diff` —
**was not built.** The thesis is half-delivered, and I would say so from the stage rather than
let someone find it.

**`Sensors::default()` is public and stamps `Instant::now()`.** So any crate can write
`ObservationAge::of(&Sensors::default())` and obtain a forged 0 ms age. A reviewer proved this
with a compiled integration test in `duck-control/tests/`, outside the module: it passes,
asserting `as_millis() == 0`. `io.rs` calls this a "wart" and explains that `bus.rs` builds one
and fills it in as it unpacks blocks. **For a talk whose thesis is "delete the field that could
be asserted," this is the hole, not a wart.** Deleting `Default` is the move the thesis demands
and it is a larger diff than the rule itself.

So, stated precisely: **the stamp is unforgeable; the freshness claim is not.**

---

## 5. Timing against the six-minute budget

| | duck (Run B) | duck (Run A) | old robosuite demo |
|---|---|---|---|
| whole arc, wall | **193.89 s** | 198 s | **76.58 s** |
| % of the 360 s budget | **53.9 %** | 55.0 % | 21.3 % |

Breakdown (Run B):

    seed tests + seeded workspace build        ~20 s
    duck-sim up (MuJoCo, policy load, stand)    16 s
    calibration sweep                           21.7 s
    seeded matrix, 7 scenarios                  58.0 s
    verify + patch + repaired tests + rebuild  ~20 s
    duck-sim up again                           16 s
    repaired matrix, same 7 scenarios           58.2 s
    final stand-up + bundle                    ~17 s

**Machine time is not stage time.** The two 58 s matrices are 116 s of the 194 with nothing for
a speaker to do but narrate. The old demo's 76.58 s has the same property — its patch-review
beat is 25 ms in the log — but 76.58 s leaves 4 m 43 s of the budget for speech and 194 s leaves
2 m 46 s. That is the real cost of the duck, and it is the main argument against running the
whole arc live.

---

## 6. What this replaces, what it does not, and what I would take on stage

**The old demo is not broken.** `rust-china-conf` passes 17/17 today with a sha256-bound
verifier on robosuite, 0 infrastructure failures, in 76.58 s. Nothing in this port degrades it.
It remains the fallback and it is a *good* fallback.

**What the duck buys that robosuite cannot:**

- The 250 ms threshold stops being disclaimed. 80 ms = 4 control periods on a loop that reports
  its own missed ticks. This directly answers the Q&A question *"is 250 ms real?"*, which
  currently has an apologetic answer.
- A genuinely structural artefact — the private `Instant` — that embodies the talk's thesis
  instead of illustrating it.
- A visible biped falling over in MuJoCo while a green health line sits next to it. That is a
  better image than a cube.
- 576 packages and a 59 s cold build, which is the workload the *build* half of the talk needs
  anyway.

**What the duck costs:**

- 194 s instead of 77 s: 54 % of the budget instead of 21 %.
- The hook changes. *"The robot succeeded and the contract failed"* is **not available on this
  duck** — the balance cliff sits below the bound, so a duck whose dispatch age clears 80 ms has
  already fallen. The talk's 0:00 and 4:00 beats are written on the old hook and must be
  rewritten (see `DUCK-TALK-BEATS.md`).
- Five unfixed staging fragilities (§7.1), none of which fired for me on a quiet machine and all
  of which fired for reviewers on a contended one.

### The recommendation, plainly

**Take the duck. Do not run the whole arc live.**

Run `scripts/duck-safety-demo` once in the green room, immediately before the session, and put
*that* bundle on screen. On stage run only the three things that are fast, independent and
essentially cannot fail:

    scripts/duck-sim ctl health          # ~1 s, a duck already standing
    scripts/duck-sim drive               # 8 s, visible walking
    cargo test -p duck-control --lib     # 0.07 s of test time, three named failures

Everything else — the matrix, both verdicts, the calibration sweep — is read from the bundle you
produced twenty minutes earlier and whose binary digests are printed on the slide. That gets you
the duck's advantages, costs under 30 s of stage time instead of 194, and removes every single
point of failure in §7.1 from the live path.

If someone insists on a live protected verdict, `verify.py` against the pre-made bundle runs in
under a second and is the same verifier — that is the beat worth doing live, not the matrix that
feeds it.

**Do not rehearse both substrates.** If the duck is the choice, retire the robosuite arc to the
fallback slide and stop maintaining two.

---

## 7. Honest limits and what is still broken

### 7.1 Five staging fragilities, all still present at `e4ae26f`

None of these is a correctness bug. All of them are ways the live arc dies on a busy machine.

1. **No mutual exclusion.** No lockfile. The cleanup trap runs
   `git checkout -- duck-control/src/safety.rs` unconditionally, so a second concurrent run
   silently reverts a patch the first one applied. A reviewer watched exactly this corrupt a
   run: a verified-patched tree reverted mid-build, producing a `robotd` *without* the gate that
   still "ran" the repaired matrix and showed zero refusals.
2. **Fixed port and fixed state directory.** Body port 7801 and `~/.cache/duck-sim` are
   hardcoded. Two runs on one machine destroy each other.
3. **Unscoped kill.** `scripts/duck-demo/run.py:113` runs `pkill -KILL -f body_server` for the
   `body_gone` scenario. That kills every MuJoCo body server on the machine.
4. **No retry on the referee.** `scripts/duck-demo/referee.py` makes exactly one
   `socket.create_connection` and raises into a hard `die`. One transient refusal from the
   single-threaded MuJoCo server during a `robotd` swap ends the whole arc. Four of one
   reviewer's five failures were this. A three-attempt retry with a 1 s gap would have saved
   them.
5. **The stand-up guard cannot fire.** `scripts/duck-safety-demo:96` gates on
   `duck-sim 2>&1 | grep -E 'standing|loop|bus|error' || die`. The banner line `== standing up`
   matches whatever happens next, *and so does the failure message* `duck-a did not stand up`.
   A reviewer watched the demo carry on and calibrate a duck that was lying on the floor. Gate
   on the `duck-a: standing` line specifically.

Fixing 1, 4 and 5 is maybe forty lines and would make the live arc worth attempting. I did not
fix them; the task was the port, and changing the harness after measuring it would invalidate
the measurement.

### 7.2 The hook the port does not have

Stated once more because it is the most consequential finding: **there is no injected delay at
which the dispatch age exceeds 80 ms and the duck still walks.** The balance cliff is between
46 and 66 ms of dispatch age; the bound is 80 ms. The harness measures this every run rather
than asserting it, and prints `calibration: no`.

Consequence the README states and I will not soften: **the shipped 80 ms gate is too lax to save
this duck under *sustained* staleness.** A caveat in the other direction, equally real: the coast
permits three *consecutive* stale ticks followed by a fresh one, which is a different load from
a sensor path that is permanently behind. These numbers do not show that `COAST_TICKS` is wrong.
They show that a bound derived from it is not a balance guarantee.

### 7.3 `borderline_40ms` is closer to the cliff than is comfortable

Measured dispatch age at 40 ms injection: 45.7 ms (Run A), 53.6 ms and 57.2 ms (Run B). The
measured balance cliff is 46–66 ms. The scenario's declared window is `[35, 75]` ms, so it will
not fail the verifier for drifting — but if this duck falls during `borderline_40ms`, the
`when_permitted` criterion (`walked`, ≥ 0.15 m) fails and the entire arc dies. **This is the
scenario most likely to end a live run,** and it is not the one anyone would be watching.

### 7.4 What "held" means, and a verifier that cannot tell the difference

`--stale-sensors-ms` is a **start-time** flag with `requires = "sim"` — deliberately not a live
knob. So the sensor path is already 100 ms behind before the duck is first enabled, the gate
refuses the very first dispatch, and the duck never rises.

    [Run B, repaired]  stale_100ms   trunk_z_start 0.0369 -> 0.0369   upright false
                       displacement 0.0011 m

The two halves are told apart by **displacement**, not by height: 0.2035 m of floor-thrashing
seeded against 0.0011 m repaired. But the verifier's `when_refused` criterion
(`outcome: held`, `max_displacement_m: 0.05`) **cannot distinguish "the gate held a standing
duck still" from "the duck was already prone and never rose."** A duck on the floor satisfies it
trivially. `trunk_z_start` and `upright_at_start` are recorded in `scenario-results.json` and
never asserted by `verify.py`. The repaired PASS on these two scenarios is therefore partly
degenerate. The bundle README discloses this; the verifier does not enforce it.

**An earlier claim that was simply wrong, and is corrected here:** the implementation summary
said the repair *"saves it at 100 ms, where it fires on 266 of 266 and the duck stays upright and
still."* It does not stay upright. Commit `e4ae26f` corrected this in the repo and the README
says so; the sentence should never be spoken.

### 7.5 The calibration is one sample per delay, and it is noisy

At 60 ms injection the measured dispatch age has come out at 63.8, 65.6, 65.9 and 86.6 ms across
four runs. The "cliff sits between 46 and 64 ms" phrasing in the Run A README is a single sample
per row. **On a slide, state the spread or average several episodes.** I have used "46–66 ms"
throughout this document for that reason.

### 7.6 Whether the seeded hook scenario starts upright is not deterministic

Run A: `stale_100ms` seeded started at `trunk_z_start 0.0552`, `upright_at_start false` — the
duck went down during the 6 s settle after the `robotd` swap, so what the trace captured was
thrashing, not a fall. Run B: it started at 0.0973, upright, and fell to 0.0484 inside its own
window. `stale_100ms_turn` started upright in both runs and fell in both.

`verify.py` imposes no `upright_at_start` requirement and cannot tell these apart. **Lead with
`stale_100ms_turn`** — it is a genuine fall in both runs on record.

### 7.7 Four claims in the code and the harness that are false as written

Found by adversarial review, confirmed, and not yet fixed. Each is the demo's own failure mode —
a rule stated in a comment and absent from the code that grades it — which makes them worth
disclosing rather than quietly patching.

1. **`check-patch.py`'s docstring overclaims.** It says the allowlist means *"'the gate now
   passes' cannot have been achieved by moving the gate, by loosening a test, or by editing the
   evidence."* It cannot prevent loosening a test: the three grading tests live *inside* the
   allowlisted file. A reviewer built a patch deleting all three (`3 90
   duck-control/src/safety.rs`) and `check-patch.py` printed `candidate allowlist passed`,
   exit 0. **The protected matrix verifier is what catches that, not the allowlist.** Say that
   instead.
2. **A drift test that cannot detect drift.** `the_default_bound_is_four_control_periods` claims
   *"if either the coast or the loop rate ever changes, this is the test that says the two have
   drifted apart."* It cannot. `duck-control` does not and structurally cannot depend on
   `robotd`; the test declares its own `const COAST_TICKS: u32 = 3` and its own 20 ms period and
   compares them to a literal `Duration::from_millis(80)`. Changing `main.rs:137` or the params
   default fails nothing anywhere.
3. **The manifest's "arithmetic" is a tautology.** `matrix.py` hardcodes `period_ms: 20`,
   `coast_ticks: 3` and `policy_observation_max_age_ms: 80` as Python literals — not read from
   the binary, the params file or a running daemon. `verify.py`'s check that *"the threshold is
   arithmetic on the control period, not a loose number"* asserts `20 × (3 + 1) == 80` between
   two constants the same script just wrote. It can never fail.
4. **`Coast::observation_age` does not return `None` past the coast.** Three places say it does.
   `Coast::sample`'s `None => None` arm never clears `self.last`, so past the coast
   `observation_age()` keeps returning `Some` with an ever-growing age. Behaviour is still safe —
   the age exceeds the bound and is refused — but the `None` arm is reachable from `robotd` only
   *before the first successful read*, so `a_tick_with_no_observation_is_refused` exercises a
   startup path, not the coast-exhaustion path its name claims.

Two smaller ones: `safety.rs` says the refusal *"commands the pose the robot is in"* — under
sustained staleness `hold` comes from the stale sample, so it is the pose the robot was in
`delay` ms ago; and `apply` calls `self.set_gain(running_gain)?` *before* the freshness check, so
a refused tick still commands the running gain. Defensible, but the doc's "first among the
refusals" is not quite the first thing `apply` does.

### 7.8 One live hazard in the verifier

`duck-ipc-proto` documents that `observation_age_us == 0` means *"the loop had no observation at
all this tick… Zero is therefore the **worst** value here, not the best."* `verify.py` reads
`t.get('observation_age_us', 0) / 1000.0` as a plain number. A zero would pull the median down
and flip `refusal_required` to `False` — the verifier would then require that the repaired gate
refused *nothing*, on exactly the ticks where it must refuse. Currently unreachable only because
`robotd` publishes a state frame solely when a sample exists, so no-observation ticks never reach
the wire. Nothing tests that. **This is the demo's own thesis failing inside the demo's own
instrument**, and it is a good thing to admit from the stage if asked.

### 7.9 Housekeeping

The Run A bundle's manifests record `git_dirty: true` at commit `7512443`, which is not the
current HEAD, and `verify.py` imposes no cleanliness requirement — so "bound to the binary" is
bound to a digest of a copy, exactly as the manifest's own caveat admits. **Run B is clean:
`git_dirty: false` at `e4ae26f` for the seeded half.** If a bundle goes on a slide, use a clean
one and say which. Two incomplete evidence directories from aborted reviewer runs are left in the
checkout under `evidence/`.

### 7.10 Not built

- `demo/structural-patch.diff` — the `Fresh<'a>` witness finale (§8d of the design). The talk's
  thesis is half-delivered without it.
- No fix for any of §7.1.

---

## 8. Exact repro from a clean machine

```bash
# 1. the two repos
git clone --depth 1 https://github.com/pollen-robotics/microduck.git
git clone --depth 1 https://github.com/pollen-robotics/microduck_rl.git

# 2. the simulator venv (MuJoCo 3.10.0 + onnxruntime 1.24.4)
cd microduck_rl && uv sync

# 3. THE GOTCHA — do not rediscover this, it cost three attempts
#    On macOS with a uv-managed Python the body server dlopens .venv/bin/python and needs a
#    shared libpython3.12.dylib. uv's standalone CPython does not expose one on the baked
#    rpath, so it fails with:  Library not loaded: @rpath/libpython3.12.dylib
#    DYLD_FALLBACK_LIBRARY_PATH does NOT help — the lookup is an @rpath resolution and dyld
#    only tries the rpath list. That list includes .venv/bin/../libpython3.12.dylib, so:
ln -sf "$(dirname "$(readlink -f "$(uv python find 3.12)")")/../lib/libpython3.12.dylib" \
       .venv/libpython3.12.dylib
#    A system or Homebrew Python 3.12 avoids the problem entirely.
#    mjpython is required only for the WINDOWED viewer on macOS; HEADLESS=1 avoids it.

# 4. apply the three local commits (this port is not upstream and must never be pushed)
cd ../microduck
git log --oneline -3      # cf5b228 seeded gate · 7512443 patch+allowlist · e4ae26f README fix

# 5. the environment every command below needs
export DUCK_SIM_RL="$PWD/../microduck_rl"
export HEADLESS=1                 # omit for a MuJoCo window
export DUCK_SIM_VIEWER=0
export PATH="$HOME/.local/bin:$PATH"

# 6. MANDATORY BEFORE A RUN — the harness has no lock (see §7.1)
scripts/duck-sim down
pkill -f body_server ; pkill -f 'target/debug/robotd'
ps aux | grep -E 'robotd|body_server' | grep -v grep     # must be empty
git status --short                                       # must be clean

# 7. the whole arc, one command, ~3 m 15 s
DUCK_DEMO_COPY_TO=/path/to/keep/it scripts/duck-safety-demo

# 8. or just the duck, for the stage beats
scripts/duck-sim                  # daemons up, duck stands
scripts/duck-sim ctl health
scripts/duck-sim drive            # walks 8 s, intent expires on its own
DUCK_SIM_STALE_MS=100 scripts/duck-sim    # the same duck, senses 100 ms behind
scripts/duck-sim log
scripts/duck-sim down

# 9. re-verify any bundle, from the checkout, in under a second
python3 scripts/duck-demo/verify.py <bundle>/seeded --expect-fail   # rc 0 means it failed
python3 scripts/duck-demo/verify.py <bundle>/repaired               # rc 0 means it passed
```

Cold full workspace build measured today: **59.23 s, 355 units, 10-core Mac.** Budget 2–3
minutes on a 4-core machine.

---

## Verdict

**The demo works. It is reproducible on a quiet machine — I reproduced it cold, first attempt,
exit 0, in 3 m 14 s. The content is stage-ready and is stronger than the demo it ports.**

**The live one-command arc is not stage-ready as a live act**, for two reasons that are both
fixable and neither of which is about correctness: it consumes 54 % of the budget, and it has
five single points of failure that fire the moment anything else touches port 7801 or
`~/.cache/duck-sim`. Run it in the green room, show the bundle, and keep the live surface to
`duck-sim drive`, `ctl health` and `cargo test`.
