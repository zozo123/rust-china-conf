# Disposable Sandboxes. Reusable Compilation.
## Final site copy — every number measured 2026-09-24, one machine, published evidence

---

## HERO

# The workspace is disposable. The compilation is not.

**An agent proposes a change in seconds. Proving it takes a build.**
That asymmetry is the whole cost of automated software research — and it is a
build problem, not a model problem.

`[ Read the evidence ]`   `[ 453 files, re-derivable from a clone ]`

---

## SECTION 1 — The loop, and where it actually costs

```
   propose  ──▶  fork a clean box  ──▶  build  ──▶  test  ──▶  judge  ──▶  keep/discard
   1 call        seconds              THE COST     THE COST     must not
                                                                be authored
                                                                by the candidate
```

Generating a candidate patch is one inference call. **Judging it costs a full
build and a full test run**, once per candidate, forever. Widen the search and
that number multiplies.

> **Cost per generation ≈ N × (build + test).**
> Everything else is rounding.

---

## SECTION 2 — Sandboxes are not a future feature. They are the method.

A candidate must not inherit the last candidate's state, or you are not
measuring the candidate. So every candidate gets a clean box.

We ran this on EC2 — four machines, one coordinator, one initiator, two
helpers. Mid-project the first grid was reclaimed and a second was provisioned
and re-keyed **in minutes**. That is not an anecdote about cloud infrastructure;
it is the property the method depends on. **A sandbox you cannot cheaply
destroy and recreate is not a sandbox.**

```
  AL6555 · Ubuntu 20.04 · 12 cores · 8 licensed
  initiator 4c/16G · coordinator 4c/8G · helper 2c/8G · helper 2c/8G
```

---

## SECTION 3 — The catch nobody prices in

**A clean box means a cold build.** Every time.

| | |
|---|---|
| build from an empty workspace | **11,515 ms** |
| …once per candidate, every candidate | |

That is the bill for isolation. Most teams pay it, notice it hurts, and quietly
stop isolating — which silently invalidates the comparisons they were isolating
for in the first place.

---

## SECTION 4 — Incredibuild Build Cache: pay it once

Same source. Same empty workspace. Cache on.

| scenario | plain cargo | Build Cache | |
|---|---|---|---|
| **empty workspace** (fork · CI · agent loop) | 11,515 ms | **3,706 ms** | **3.13× faster** |
| warm workspace (a developer rebuilding) | **939 ms** | 4,048 ms | cargo wins |

**Both rows are published, because the second one is how you know the first is
honest.**

Proof of participation, not inference — from Incredibuild's own report:

```
HIT lines : 52
MISS lines: 0
tasks executed: 9 of 58      # 49 compilations never invoked rustc
helpers used  : 0            # cache only, no distribution
```

> **The Build Cache does not make your build faster.
> It makes throwing your workspace away cheap.**

That is exactly the operation a sandboxed agent loop performs on every single
candidate.

### What we do not claim

Distribution did not help this workload, and we publish that too: `rustc` is one
process per crate and the crate graph is deep rather than wide, so the warm
one-file rebuild parallelises only **1.08×** from `-j1` to `-j10`. There is no
parallelism left to sell. Cross-machine cache sharing is unconfigured on this
grid and therefore undemonstrated.

---

## SECTION 5 — Rust, where it matters

The thing being judged is a **safety contract on a robot**: an observation older
than the policy bound must never authorize motion.

The enforcement is not a runtime check. It is the type system:

```rust
Sensors { observed_at: Instant }   // private — Instant::now() is the only
                                   // constructor. A backend carries a stamp,
                                   // it cannot write one.

ObservationAge                     // no Default, no From<Duration>, no public
                                   // field. ObservationAge::of(&Sensors) is
                                   // the only way in.

Safety::apply(.., age: Option<ObservationAge>)
                                   // by value, by type: a dispatch site with
                                   // no observation DOES NOT COMPILE.
```

**"I checked the age" stops being a claim a caller can make.**

On a 50 Hz control loop the bound is arithmetic, not opinion:
**80 ms = 20 ms × 4 control periods.**

---

## SECTION 6 — A reward the candidate cannot write

This is the part that decides whether an automated loop converges on better
software or on better-looking receipts.

We learned it the hard way. Our own benchmark verifier certified four
fabrications before it stopped:

| | it printed | why |
|---|---|---|
| a receipt asserting its own cache state | `ratio=2.999x` | it read a literal the producer wrote |
| a digest checked for shape only | `ratio=11.948x` | the file was never opened |
| ten **real** transcripts around a no-op tool | `ratio=119.949x` | real documents, wrong authorship |
| a coverage matrix testing 0/50/600 ms | "complete coverage" | never tested its own 250 ms boundary |

**42 guard tests. Twelve forgery classes closed. One residue, printed by the
tool in its own output**, because a verifier that hides its blind spot is worse
than one that names it.

> **A check wins against exactly the adversary who does not control its input.**

Which is why the reward must be **re-derived**, never reported: the verifier
recomputes the verdict from recorded traces against a binary bound by sha256.
You cannot hand it an answer.

And why a cache hit is trustworthy where a wall-clock is not — **you cannot
author a cache hit.** The key is computed from input content by something that
is not you.

---

## SECTION 7 — What this is

```
   Rust            the contract, enforced by types rather than vigilance
   Sandboxes       a clean box per candidate — EC2 today, same shape anywhere
   Build Cache     3.13x, which is what makes a clean box affordable
   Verification    a reward the candidate cannot author
```

Four parts. Remove any one and the loop degrades into something that looks like
research and measures nothing.

`[ Evidence: 453 files ]`  `[ The talk ]`  `[ The proof layer ]`  `[ The port ]`

---

## FOOTER — honest limits

Measured on one 45-package Rust workspace, one machine, n=5 per mode, rotating
order. Larger-workload results pending. Cross-machine cache sharing not
demonstrated. No distribution speedup is claimed; the measured distribution
result was negative and is published. Three of the numbers on this page replaced
earlier numbers we withdrew — the corrections are in the repository with the
measurements that forced them.
