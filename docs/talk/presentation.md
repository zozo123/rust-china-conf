# How Do You Know?
# 你怎么知道？
### A robot that succeeded while it was wrong, a proof gate that certified four fabrications, and the one rung that held

**Rust Conf China · 25 minutes**
*Subtitle strip, always visible: 250 ms · 20 ms · 11,515 ms — all measured 2026-09-24*

---

# THE ARGUMENT, IN ONE PAGE

Every layer of automation you add moves the question *"how do you know?"* one level
down. At each level the cheapest way to **look** correct is to assert correctness
rather than earn it. And as you add optimisation pressure — an agent, a loop, a
deadline — asserting gets *cheaper relative to earning*, not more expensive.

```
  the robot        asserts   "the task completed"
  the test suite   asserts   "the contract holds"
  the receipt      asserts   "the cache was cleared"
  the agent will   assert    "the reward was achieved"
```

Four claims. Four authors. **In every case, the author of the claim was also the
party the claim was about.**

The talk's answer is not "verify harder". Verification is itself a claim. The
answer is one sentence:

> ## A check wins against exactly the adversary who does not control its input.

That is why the type system works and our verifier didn't. `rustc`'s adversary is
the next contributor — who controls the diff and **cannot author `rustc`**. Our
verifier's adversary was *me*, and I owned its input, so no amount of structure
could have saved it.

**Rung 4 — evidence you did not author — appears nowhere in this repository
except the compiler. And, it turns out, the build cache.**

---

# ACT I — THE ROBOT (0:00 – 6:00)

## 0:00 · Cold open. No title slide.

**LIVE**
```bash
scripts/duck-sim drive
```
```
== walking forward for 8 s
loop  50.0 of 50.0 Hz · 721 ticks · 3 missed · last 0 ms ago
bus ok · imu ready · battery 7.40 V · motors 32 °C
```

**SAY**
> "The robot lifted the cube. It succeeded. And it was wrong the whole time —
> I'll show you how I know, and then I'll show you why that 'how I know' isn't
> good enough either."

*Fallback: the `scenario-results.json` row itself on screen. Not the mp4 — that
footage is a fresh-lift episode, not the stale one.*

## 2:00 · The number under the green light

```
 50 Hz control loop:   |-20-|-20-|-20-|-20-|-20-|-20-| ... ms
 contract threshold:   |<------- 250 ms = 12.5 ticks ------->|
 what it acted on:     |<--------------- 600 ms = 30 ticks ------------->|
```

> "Six hundred milliseconds old. The loop runs at twenty. Thirty control periods.
> The age was in the data. It stepped anyway."

## 4:00 · Task green, contract red

**LIVE**
```bash
cargo test -p robot-safety-gate --test contract
```
```
test boundary_251ms_rejects ... FAILED
test stale_ages_are_rejected ... FAILED
test configured_threshold_is_respected ... FAILED
test result: FAILED. 5 passed; 3 failed
```

**ON SCREEN** — `robot-safety-gate/src/lib.rs:147`
```rust
let _ = (age_ms, policy);
```

> "Rule three: observation age under 250 ms at dispatch. This is the shipped
> implementation of rule three. The age arrives, and we throw it away.
>
> One line. It silences the unused-variable warning. That is the whole bug.
>
> **Task success is one check. It is not the check.**"

---

# ACT II — THE LADDER (6:00 – 17:00)

## 6:00 · Rung 1 → Rung 2

> "So we wrote a contract, and tests to pin it. Three of them catch this.
>
> But look at who wrote the tests, and who wrote line 147. **Same person. Same
> afternoon.** The contract's adversary controls the contract's input."

## 8:00 · Rung 3 — we built a proof gate, and it lied. Four times.

**Forgery one — the receipt certified itself**
```
cache_cleared_before_each_cold_sample: true     <- a literal the producer wrote
$ build-proof --receipt fabricated.json
BUILD PROOF PASS ... ratio=2.999x vs native; saved=14000ms      exit 0
```
> "Three of eight fail-closed conditions could never fire."

**Forgery two — we fixed it, and moved the hole**
```
transcript_path:   "/tmp/does-not-exist.txt"
transcript_sha256: "0000…0000"          <- 64 hex chars. That was the entire check.
-> BUILD RECEIPT CONSISTENT ... ratio=11.948x                   exit 0
```
> "We validated that the digest was sixty-four hexadecimal characters. We never
> opened the file."

**Forgery three — real files, real digests, still a lie**
```
ten transcripts, produced by the repo's OWN cache-clear.sh,
wrapped around a three-line script that prints "namespace user purged"
and touches nothing.
-> all ten opened. all ten re-hashed. "corroborated."
-> ratio=119.949x                                               exit 0
```
> "This is the one that changed my mind. Real documents. Real digests. The tool
> did everything it promised. **And I wrote both sides.**"

**Forgery four — found this week, in the demo's own coverage claim**
```
coverage-matrix.json :  "freshness_ms": [0, 50, 600]
lib.rs:25            :  DEFAULT_MAX_OBSERVATION_AGE_MS = 250
```
> "Seventeen scenarios. The verifier calls it a *complete coverage matrix*. Any
> staleness above 500 ms passes all seventeen. The boundary is pinned only in a
> test file nobody puts on a slide."

## 14:00 · And then I did it to myself. Twice.

**ON SCREEN**
```
-f, --force-remote   force allow_remote tasks to remote helpers
                     ^ ib-benchmark.sh:176
max_initiator_cores = 0        <- four local cores. Idle. Every run.
```
> "I published 'the distributed build is 2× slower'. Then I read one line of help
> text. I had not measured a build system. I had measured four remote cores
> **replacing** four idle local ones."

**AND THE ONE I LIKED MOST**
> "I then derived a beautiful number — 87% parallel efficiency — and built a slide
> around it. It divided *helper* CPU-seconds by a *different machine's* wall time,
> and then by a core count that exists nowhere on disk.
>
> It was wrong. It felt like insight. **That is what a claim you authored feels
> like from the inside.**"

---

# ACT III — WHAT SURVIVED (17:00 – 21:30)

## 17:00 · Measure one machine, one tool, or don't bother

```
              cold-j1   22,861 ms      cold-j10   6,976 ms     3.28x
              warm-j1    1,016 ms      warm-j10     942 ms     1.08x
                                              n=5 each, one machine
```
> "Cold builds saturate at about 3.3× no matter how many cores you add. And the
> build that actually matters in a loop — change one file, rebuild — is **1.08×.
> Effectively serial.**
>
> Distribution's entire product is parallelism. **This workload has none left to
> sell.** That is not a criticism of any vendor. `rustc` is one process per crate;
> the graph is deep, not wide."

## 19:00 · So stop distributing. Cache.

**ON SCREEN** — one word changed in the profile
```xml
<process filename="rustc" type="local_only">   <!-- was: allow_remote -->
  <ib_cache enabled="true" />
</process>
```

```
  MODE                              MEDIAN      HITS     vs NATIVE
  native cold                      11,515 ms      —         —
  cache-only, COLD cache           16,049 ms    1/52     0.72x  (+4.5 s)
  cache-only, WARM, one file       6,527 ms    47/52     1.76x  (-5.0 s)
  cache-only, WARM, full reuse     3,706 ms    52/52     3.13x  (-7.9 s)

  n=5 per mode · rotating order · remote_tasks=0 on all 21 builds
  build-proof --distribution excluded : BUILD RECEIPT CONSISTENT
```

> "Cold cache **costs** you four and a half seconds. You pay to fill it. Then you
> get three times back.
>
> I'm showing you the price because a speedup with no price attached is the kind
> of claim this talk is about."

**Two gifts for your Monday**
```
  debug=0            10,399 ms   1.12x   (still 2.81x behind full reuse)
  explicit lld       11,619 ms   0.999x  — rustc 1.92 ALREADY defaults to LLD 21.1.3
```
> "Half this room has 'switch to lld' in a backlog. On 1.92 you already have it."

## 20:30 · The rung that held

> "Why should you believe the cache number when you should not believe my ratio?
>
> **Because I cannot author a cache hit.** The key is computed from the content of
> the inputs, by something that is not me. Fifty-two of fifty-two is a claim the
> system makes about itself. If I could forge it, the build would be *wrong*, and
> that is a different and much louder failure.
>
> The wall-clock in my receipt is my word. The hit count is not."

---

# ACT IV — CLOSE (21:30 – 25:00)

```
   policy   : gate        ::  candidate : verifier   ::  job : coordinator
   proposes : disposes        proposes  : re-derives     asks : records
```

> "Three systems, one shape. The policy holds no IO handle — it *cannot* command a
> motor. The verifier re-derives from traces and cannot be handed an answer. The
> coordinator writes the build record; the job cannot author its own counters.
>
> **In each one, the thing that proposes is not the thing that attests.** Every
> failure I showed you is that separation leaking — including the two I committed
> myself, with a command-line flag and a flattering arithmetic.
>
> And the rule that tells you which checks are worth writing:
>
> ## A check wins against exactly the adversary who does not control its input.
>
> Your type system works because the next contributor cannot author `rustc`. My
> verifier failed because its adversary was me.
>
> We built a machine to catch unverified claims. We pointed it at our own work and
> it caught us four times, in its own code.
>
> **That is not this talk going wrong. That is the only reason you should believe
> any number in it.**"

---

# SLIDE LIST (15)

```
 1  the duck, live                          9  forgery 4: coverage matrix
 2  20 ms / 250 ms / 600 ms timeline       10  -f, --force-remote
 3  contract test failure                  11  my 87% was wrong
 4  let _ = (age_ms, policy);              12  j1 vs j10: 3.28x / 1.08x
 5  same author, same afternoon            13  one word: local_only
 6  forgery 1: the literal                 14  the cache table + its price
 7  forgery 2: 64 zeros                    15  proposes : attests
 8  forgery 3: real files, real digests        + the one sentence
```

# Q&A — the eight you will get

1. **"Did you ship a broken verifier?"** Yes, twice. It is fixed for twelve attack
   classes and open for one, which it prints itself. A verifier that hides its
   residue is worse than one that names it.
2. **"Why trust any number here?"** Don't. Clone it. Every figure has a command.
3. **"Isn't the distribution result a bad config?"** Partly — `-f` made it worse.
   But 1.08× on the warm rebuild caps the honest upside regardless.
4. **"Is 250 ms real?"** On the old harness it was illustrative. On a 50 Hz loop
   it is 12.5 control periods, and the loop reports its own missed ticks.
5. **"Is this two talks?"** Both halves are one failure: a claim that looks
   verified and isn't. The robot supplies stakes; the build supplies rigour.
6. **"Why not sccache?"** Unmeasured here — say so. The architectural point is
   independent of vendor: the coordinator attests, the job does not.
7. **"Is the seeded bug contrived?"** `let _ = (age_ms, policy);` is what a tired
   person writes to silence a warning. That is exactly why it is frightening.
8. **"What would close the last forgery?"** Sign the receipt with a key the
   producing job never holds. Separate the builder from the attestor.

# NEVER SAY

| ✗ | ✓ |
|---|---|
| "87% parallel efficiency" | **deleted — the derivation was invalid** |
| "21.426 vs 21.948" | different machines, single sample |
| "88/88 checks passed" | today's verifier reports **17 scenarios** |
| "build-proof proves it" | it checks consistency; one forgery still passes |
| "Incredibuild is 2× slower" | "…with `--force-remote`, which idled four local cores" |
| "complete coverage matrix" | say **17 scenarios at 0/50/600 ms** |

# BLOCKING, BEFORE STAGE

```
[ ] evidence/*/ is gitignored (.gitignore:19) -> no run id is checkable from
    the audience's seat. This guts "clone it and rerun it", which is the talk's
    entire credibility mechanism. Commit the bundles or publish them.
[ ] demo/forged/ does not exist -> the forgery beats have nothing to run against.
[ ] pick ONE robot substrate. The robosuite harness has 17/17 and a sha256-bound
    verifier today; the duck has a 50 Hz loop and a better story. Do not rehearse
    both.
```
