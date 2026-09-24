# Storyboard — "How Do You Know?"
## Rust Conf China · 25 minutes · one question, asked five times

---

## THE SPINE

The talk is one question asked at five increasing depths. Each time, the honest
answer turns out to be "I didn't." The floor gives way five times, and the fifth
time it gives way under *the speaker*.

```
  HOW DO YOU KNOW THE ROBOT IS SAFE?
   └─> the task succeeded .................. X  the contract failed
       |
       HOW DO YOU KNOW THE GATE IS RIGHT?
        └─> the contract tests pass ........ X  rule 3 was never written
            |
            HOW DO YOU KNOW THE BENCHMARK IS REAL?
             └─> the proof gate said PASS ... X  it certified a fabrication
                 |
                 HOW DO YOU KNOW THE PROOF GATE WORKS?
                  └─> we tested it .......... X  8 forged receipts passed
                      |
                      HOW DO YOU KNOW IB IS SLOWER?
                       └─> we measured it ... X  we measured it with --force-remote
```

**Every rung is a check. Checks compose downward forever.**
The talk's answer is not "verify harder". It is the rung where the regress stops:

```
   CHECK                             STRUCTURE
   "is the field true?"     vs       the field does not exist
   "did it command a motor?"vs       it holds no IO handle
   "is this borrow safe?"   vs       it does not compile
```

> That is not a testing lesson. That is the idea Rust is built on,
> arriving from the outside, uninvited, in a benchmark receipt.

---

## THE ARC

```
 min  0    4        7          11              16         19        23   25
      |----|--------|----------|---------------|----------|---------|----|
      STAKES CONTRACT  MACHINERY    THE DESCENT   OUR TURN  RESOLUTION CLOSE
       duck   red      gate+proof   3 forgeries    -f       structure
       walks  verdict               of our own     87%      > check
       stale                        verifier
      \____________________/\___________________/\____________________/
         ACT I: the hook       ACT II: the fall      ACT III: the floor
         "it worked and        "the tool that        "where the regress
          it was wrong"         checks was wrong"     actually stops"
```

---

## ACT I — THE HOOK (0:00–7:00)

```
  +--------------------------------------------------------------+
  |  LIVE: scripts/duck-sim drive                                |
  |                                                              |
  |      == walking forward for 8 s                              |
  |      loop 50.0 of 50.0 Hz - 721 ticks - 3 missed             |
  |                                                              |
  |  The duck walks. Every light is green.                       |
  +--------------------------------------------------------------+
                              |
                              v
  +--------------------------------------------------------------+
  |  It is walking on an observation that is 600 ms old.          |
  |                                                              |
  |  50 Hz control loop:  |-20-|-20-|-20-|-20-|-20-| ... (ms)    |
  |  contract threshold:  |<------ 250 ms = 12.5 ticks ------>|  |
  |  what it acted on:    |<----------- 600 ms = 30 ticks ------ |
  |                                                    ------->| |
  |                                                              |
  |  Thirty control periods. It knew, and it stepped anyway.      |
  +--------------------------------------------------------------+
                              |
                              v
              PROTECTED VERDICT: FAIL
              outcome 'walked' != expected 'rejected_stale'

              THE ROBOT SUCCEEDED.  THE CONTRACT FAILED.
```

**Opening line:** *"This robot is working perfectly. I'm going to spend the next
twenty-five minutes explaining why that isn't good enough."*

---

## ACT II — THE DESCENT (7:00–19:00)

Each panel is a rung. The audience should start to see the pattern before the
speaker names it.

```
  RUNG 1 --------------------------------------------------------
  We wrote a gate.   decide(&Proposal, &Policy) -> Decision
  Rule 3: observation age <= 250 ms at dispatch.
  Shipped code, line 147:      let _ = (age_ms, policy);
                               ^^^^^^^^^^^^^^^^^^^^^^^^^
                               the age arrives and is thrown away
  cargo test --test contract -> 5 passed; 3 FAILED

  RUNG 2 --------------------------------------------------------
  So we built a proof gate for our benchmarks.
  It read:  cache_cleared_before_each_cold_sample: true
  ...which the producing script had just written as a literal.
  The receipt certified itself.

      $ build-proof --receipt fabricated.json
      BUILD PROOF PASS ... ratio=2.999x vs native; saved=14000ms
      exit 0

  RUNG 3 --------------------------------------------------------
  We fixed it. Transcripts now, not literals.
  The validator checked the digest was 64 hex characters.
  It never opened the file.

      transcript_path:   "/tmp/does-not-exist.txt"
      transcript_sha256: "0000000000000000...0000"
      -> BUILD RECEIPT CONSISTENT ... ratio=11.948x   exit 0

      Nothing was invented but JSON fields.

  RUNG 4 --------------------------------------------------------
  We fixed it again. Open the file. Recompute the digest.
  An adversary forged eight more.
  One still passes. Today. In the repo. Right now.

  RUNG 5 --------------------------------------------------------
  And the measurement itself:

      native    11,518 ms  |########
      ib-cold   23,173 ms  |################    2.01x SLOWER
      ib-warm   22,297 ms  |###############     1.94x SLOWER

  We published that. Then we read one line of --help:

      -f, --force-remote   force allow_remote tasks to remote helpers
                           ^ ib-benchmark.sh:176 passes this

      maxInitiatorCores = 0     <- four local cores, idle, all run
```

> **We had not measured Incredibuild. We had measured four remote cores
> replacing four idle local ones.**

### The number that predicted it, before we ran anything

```
   40 s compile CPU  (remoteCoreTime, from IB's own telemetry)
   -----------------------------------------------------------  = 3.47 cores
   11.518 s native wall

   3.47 / 4 cores = 87% parallel efficiency

   cargo was already at 87% of perfect.
   To win, a distributor had to beat 3.47 effective local cores
   NET OF shipping multi-megabyte rlibs over a network.

   rustc = one process per crate.  The graph is DEEP, not WIDE.
   Rust is close to distribution's worst case, and the arithmetic
   said so before the benchmark did.
```

---

## ACT III — WHERE THE FLOOR IS (19:00–25:00)

```
  Which fixes actually held?

  +----------------------------------+-------------------------------+
  |  WE ADDED A CHECK                |  WE REMOVED THE POSSIBILITY   |
  +----------------------------------+-------------------------------+
  |  read the boolean field          |  DELETE the field. The claim  |
  |    -> still forgeable            |  cannot be asserted at all.   |
  |                                  |                               |
  |  validate the digest's shape     |  derive it from a transcript  |
  |    -> still forgeable            |  whose absence FAILS CLOSED   |
  |                                  |                               |
  |  "must not command a motor"      |  hold NO IO HANDLE. It        |
  |    -> a rule to remember         |  *cannot*. (microduck's own   |
  |                                  |   control.rs says this)       |
  +----------------------------------+-------------------------------+

         checks compose downward forever
         structures terminate the regress
```

> **The regress stops not where you stop verifying,
> but where you stop *representing* the thing that could be false.**

And the counter-example that keeps it honest:

```
  Still forgeable today: the Build History counters.
  Why? Because they are still merely ASSERTED --
  no retained source document, nothing to re-derive them from.

  The transcripts are no longer forgeable.
  The counters are.
  The difference is not how hard we checked. It is what we kept.
```

**Closing line:** *"We built a machine to catch unverified claims.
It caught ours — three times, in its own code. That is not this talk
going wrong. That is the only reason you should believe any number I
just showed you."*

---

## THE TWO HALVES, BOLTED BY ONE IDEA

A reviewer's first objection will be "this is two talks". It isn't — but the
seam has to be visible:

```
        ROBOTICS                          BUILD ECONOMICS
   a gate that decides              a receipt that certifies
   whether to move metal            whether a number is real
            \                              /
             \                            /
              +----------+  +------------+
                         |  |
                    SAME FAILURE MODE:
              a claim that LOOKS verified and isn't
                         |  |
              +----------+  +------------+
             /                            \
   250 ms = 12.5 ticks              87% parallel efficiency
   (derived, not chosen)            (derived, not chosen)
```

The robot supplies the *stakes*. The build supplies the *rigour*.
Neither works alone: a safety talk with no measurement is vibes,
a benchmark talk with no consequence is a vendor slide.

---

## LIVE BEATS AND THEIR FALLBACKS

```
  #   BEAT                          LIVE COMMAND                  FALLBACK
  --  ----------------------------  ----------------------------  -----------------
  1   duck walks                    scripts/duck-sim drive        recorded gif
  2   ...on a 600 ms observation    ctl health (ticks/missed)     slide of output
  3   contract fails                cargo test --test contract    slide (5 pass/3 fail)
  4   forged receipt passes         build-proof --receipt fake    slide + exit 0
  5   the -f reveal                 ib_console --help | grep -f   slide
  6   87% derivation                done on a whiteboard slide    n/a (it's arithmetic)
  7   repaired gate refuses         re-run stale scenario         slide of verdict
```

Rule for every live beat: **if it fails, say "this is why I brought a
recording" and move on in under five seconds.** Do not debug on stage.
The talk is about claims that don't survive scrutiny; a speaker who
fumbles gracefully is on-message.

---

## [PENDING] — numbers not yet measured

```
  [ ] cache-only warm wall time            -> Act II, the cache panel
  [ ] hit fraction after one-file change   -> Act III, the agentic-loop slide
  [ ] microduck build on the 4-core grid   -> Act II, workload comparison
  [ ] duck demo full arc wall time         -> Act I, stage budget check
```

Each of these has a slide waiting for it. **If the number does not arrive,
the slide does not ship** — the talk says so out loud, which is itself
on-message.
