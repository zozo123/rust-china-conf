# The Factory
## Building · forking the platform · a reward that cannot be written

---

## The idea in one line

> **An agent that can write its own reward will write it.
> We proved that on ourselves before any agent got the chance.**

A fabricated benchmark receipt *is* reward hacking. Our `build-proof` certified
`ratio=11.948x` from a receipt naming `/tmp/does-not-exist.txt` with a digest of
sixty-four zeros. No agent did that. We did, by accident, twice — which is the
best possible evidence that a research loop needs a reward it *cannot author*.

That single observation joins the three halves of this work:

```
   ROBOT                    FACTORY                   REWARD
   a gate decides      N candidates, each         a verdict that is
   whether to move  ->  forked, built, tested  ->  RE-DERIVED, never
   metal                and scored                 reported
        \                     |                        /
         \                    |                       /
          +-------- the same failure mode -----------+
             a claim that LOOKS verified and isn't
```

---

## The loop

```
            +---------------------------------------------------+
            |                  THE FACTORY                      |
            +---------------------------------------------------+

   [ Pi / gateway ]        propose N candidate patches
          |                (bounded: allowlist = 1 file)
          v
   +--------------+  fork   +--------+ +--------+ +--------+
   |  base commit | ------> | box A  | | box B  | | box C  |   ... N
   +--------------+         +--------+ +--------+ +--------+
                                |          |          |
                       build (IB cache: disposable workspace,
                              reusable compilation)
                                |          |          |
                                v          v          v
                             test + PROTECTED VERIFIER
                                |          |          |
                                v          v          v
                          reward = re-derived verdict
                                \         |         /
                                 +--------+--------+
                                          |
                                    keep / discard
                                          |
                                    next generation
```

**The bottleneck is evaluation, not generation.** Generating a patch is one
inference call. *Judging* it costs a build and a test run. N candidates × a full
rebuild is the entire cost of the loop — which is exactly where build caching
stops being a nice-to-have and becomes the thing that makes the method possible.

```
   cost per generation  ~  N x (build + test)
                              ^^^^^
                              this is what the cache attacks

   measured today:   rust-china-conf  45 pkgs   11.5 s cold
                     microduck       576 pkgs   59.2 s cold (10 cores)
   with reuse:       fixed path, contents wiped -> 52/52 units cached
```

Pillar 3 of the existing talk — *"disposable workspaces, reusable compilation"* —
stops being a slogan the moment you run a factory. A fork **is** a disposable
workspace. The cache **is** the reusable compilation.

---

## Why the reward must be structural

A reward the candidate can influence is a reward the candidate will optimise
*instead of* the task. Our own three failures are the taxonomy:

```
  FAILURE                         WHAT THE AGENT WOULD DO
  ------------------------------  --------------------------------------
  receipt asserts its own state   write "cache_cleared: true"
  digest checked for shape only   point at a file that does not exist
  counters with no source doc     invent remote_tasks=412

  THE FIX IS NEVER "CHECK HARDER":
    delete the field         -> the claim cannot be expressed
    re-derive from transcript-> absence fails closed
    bind to sha256 of binary -> the verdict names what produced it
```

The protected verifier already has the right shape: it **re-derives** the verdict
from recorded traces against a manifest bound to a sha256-identified executable.
You cannot hand it a number. That is what makes it a reward and not a suggestion.

> **Reward hacking is not an AI problem. It is an unverified-claim problem
> that happens to have an optimiser pointed at it.**

---

## What runs where

```
  AL6555 grid (12 cores, ~30G RAM, ~99G disk)
  +---------------------------------------------------------------+
  | INITIATOR 10.133.27.234  4c/16G/27G   fork host + IB initiator |
  | COORDINATOR 10.133.18.198 4c/8G/40G   IB coordinator + cache   |
  | HELPER 10.133.30.40       2c/8G/16G   compile capacity         |
  | HELPER 10.133.3.75        2c/8G/16G   compile capacity         |
  +---------------------------------------------------------------+
  ~6 concurrent forked workspaces before RAM is the limit.

  Mac: microduck + MuJoCo duck-sim (the robot half, already running)
  Databricks Unity Gateway: inference for candidate generation
```

---

## Smallest honest demo

Not a full RL system. One generation of the loop, measured end to end:

```
  1. seed     the freshness rule is omitted; duck walks on a 600 ms observation
  2. propose  Pi generates 4 candidate patches (allowlist: one file)
  3. fork     4 boxes from the same base commit
  4. build    IB cache; record hits/misses per fork
  5. judge    protected verifier re-derives each verdict
  6. report   which candidates passed, what each build cost,
              and what the cache saved across 4 forks of the same base
```

That yields three numbers worth a slide: **cost per candidate**, **cache hit rate
across sibling forks**, and **how many candidates the verifier refused**.
The third is the interesting one — it is the reward doing its job.

---

## Still needed from you

- **Databricks workspace hostname** (e.g. `dbc-xxxx.cloud.databricks.com`).
  The PAT alone cannot address the gateway; endpoints are
  `https://<workspace>/ai-gateway/anthropic` etc. Token is stored `0600`
  outside the repo; **rotate it**, it was pasted into chat.
- Confirmation that the grid is ours for the window — sibling forks sharing one
  IB cache is the point, but another tenant's builds would pollute the hit rates.
