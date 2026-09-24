# Duck beats — how the microduck port slots into the talk

Replacement beats for **ACT I — THE ROBOT (0:00 – 6:00)** of `PRESENTATION.md`, plus the four
places later in the talk that change as a consequence.

Every number below is measured and appears in `DUCK-DEMO.md` with its provenance. **Nothing here
is estimated, rounded up, or reused from the robosuite demo.** Two runs exist; where a number
differs between them I say which run, and where it moves between runs I give the spread.

**The staging rule this plan assumes:** run `scripts/duck-safety-demo` once in the green room
immediately before the session and put *that* bundle on screen. Live on stage: `duck-sim ctl
health`, `duck-sim drive`, and `cargo test -p duck-control --lib`. Nothing else. Reasons in
`DUCK-DEMO.md` §5 and §7.1 — the full arc is 193.89 s, 54 % of a six-minute budget, and has
five single points of failure that fire the moment anything else on the machine touches port
7801.

---

## What changes, in one table

| beat | was | becomes |
|---|---|---|
| 0:00 cold open | duck walks; "it lifted the cube" | duck walks; **"in a minute I'm going to make it fall over, and every light stays green"** |
| 2:00 the number | 250 ms / 600 ms, illustrative | **80 ms = 4 control periods**, arithmetic |
| 4:00 task green, contract red | `robot-safety-gate`, 3 failures | `duck-control`, **the same 3 failures, same shape** |
| 4:30 the line | `let _ = (age_ms, policy);` | **`let _ = age;`** at `safety.rs:423` |
| — | *(new)* | **5:15 the repair, and what "held" actually looks like** |
| Act IV close | "the policy holds no IO handle" (our design) | the same sentence, **quoted from microduck's own source** |

---

## ACT I — THE ROBOT (0:00 – 6:00), rewritten

### 0:00 · Cold open. No title slide. LIVE.

**SCREEN** — terminal, full bleed. MuJoCo window beside it if the room's projector takes two
sources; otherwise terminal only.

```bash
scripts/duck-sim ctl health
```
```
robot     healthy
  loop      49.8 of 50.0 Hz · 345 ticks · 0 missed · last 20 ms ago
  bus       ok
  imu       ready
  battery   7.40 V (50%)
  motors    32 °C max
```
```bash
scripts/duck-sim drive
```
```
== walking forward for 8 s
== stopped — the intent expires on its own
```

**SAY**

> "That's a biped robot. Fifteen servos, a fifty hertz control loop, a policy trained in MuJoCo
> and exported to ONNX. Everything on this screen is true.
>
> In about four minutes I'm going to break one line of its safety code — one line, the kind you
> write to silence a compiler warning — and I'm going to make it fall on its face.
>
> And this screen will look exactly like it does now. Healthy. Fifty hertz. Bus ok. IMU ready.
>
> That's the talk. Not 'robots are hard'. **The instruments agreed, and they were all reading
> the right numbers.**"

*Timing: 25 s of machine, ~35 s of speech. Fallback if `drive` misbehaves: the `ctl health`
line alone carries the beat.*

---

### 1:00 · Whose robot is this

**SCREEN** — one slide, four lines.

```
microduck · pollen-robotics · Apache-2.0
23 workspace members · 576 locked packages · 117k lines of Rust
robotd: 50 Hz control loop, 15 servos, JSON-RPC over a Unix socket
cold full build: 59.23 s, 355 units, 10-core Mac
```

**SAY**

> "Not my robot. Somebody else's, off GitHub, and I want you to notice that — because every
> structural thing I'm about to praise was already in it before I got there. I added
> twenty-two lines and a bug."

*15 s. This slide also pre-loads the build half of the talk: 576 packages is the workload Act
III measures.*

---

### 2:30 · The number under the green light

**SCREEN** — the timeline, replacing the old 20/250/600 strip.

```
 50 Hz control loop:      |-20-|-20-|-20-|-20-| ms
                           ^^^^ the policy is trained on data exactly one tick old
 COAST_TICKS = 3:              |----- 60 ms the coast already survives -----|
 the bound:               |<------- 80 ms = 4 control periods ------->|

 what it dispatched on:   |<---------- 109.7 ms ---------->|
```

**SAY**

> "Eighty milliseconds. I want to be careful about where that number comes from, because a
> threshold on a slide is exactly the kind of claim this talk is about.
>
> It is not a number I chose. The period is twenty milliseconds — that's the loop, and the loop
> reports its own missed ticks, so I'm not asserting it, it is. The repo's own comment says the
> policy is trained on data exactly one tick old. And the repo's own constant says three further
> ticks are survivable — *'a drop, a retry and a slow tick; past that the robot genuinely cannot
> see, and holding still is the honest answer.'* Their words, not mine.
>
> Three plus one is four. Four periods is eighty milliseconds. **It's arithmetic on a loop that
> measures itself.**
>
> It dispatched on a hundred and ten."

**Speaker note — this is the beat that fixes a standing weakness.** In the old harness 250 ms
was disclaimed in the source as *"an illustrative policy, not a hardware limit."* That
disclaimer is gone. If someone asks "is the threshold real?", the answer is now a derivation,
not an apology.

*40 s.*

---

### 4:00 · Task green, contract red. LIVE.

**SCREEN** — terminal.

```bash
cargo test -p duck-control --lib
```
```
test safety::tests::a_stale_observation_holds_the_pose ... FAILED
test safety::tests::a_tick_with_no_observation_is_refused ... FAILED
test safety::tests::the_freshness_bound_is_the_configured_one ... FAILED

test result: FAILED. 80 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

**SAY**

> "Eighty pass. Three fail. And the three that fail are the three about freshness."

*Runs in 0.07 s of test time. This is the single safest live command in the talk and it is worth
doing live for exactly that reason.*

---

### 4:30 · One line

**SCREEN** — `duck-control/src/safety.rs`, lines 414–423, the doc comment **above** the line
kept on screen. This is the whole slide; do not shrink the comment to fit the code.

```rust
// Rule 1 of three: the sample these targets were computed from must still be worth
// acting on. Past `observation_max_age` — four control periods, the bound `COAST_TICKS`
// already implies — the robot is driving a world it has not seen for four ticks, and
// `hold` is the honest answer: the same answer the coast gives when it runs out of
// ticks, for the same reason.
//
// First among the refusals because it invalidates the two below it: whether a target
// is finite or inside the actuator's travel is not an interesting question about a
// target computed from a stale sample.
let _ = age;
```

**SAY**

> "There's the rule. Written out. Four control periods, justified, cross-referenced to the
> coast. Somebody clearly understood it.
>
> And there's the implementation. `let _ = age`. The age arrives at the dispatch point and we
> throw it on the floor. It silences the unused-parameter warning. **That is the entire bug.**
>
> Now — what would you have found if you'd reviewed this? The refusal reason exists:
> `Limit::Stale`. Its name is on the IPC wire: `stale_observation`, next to `deadman`. The
> config field exists. It's registered in the params file as
> `safety.observation_max_age_ticks`, default four, with a description. There is a unit test
> file that names the rule three times.
>
> **Every artifact a reviewer inspects says the rule is there. The only thing missing is the
> comparison, and the comparison is the one thing no artifact shows.**"

*45 s. This is the strongest slide in Act I and it should get silence after it.*

---

### 5:15 · The fall. Pre-recorded bundle, read live.

**SCREEN** — two panes, side by side, from the green-room bundle. Left: the referee trace. Right:
`health.json` for the same scenario, verbatim.

```
scenario  stale_100ms_turn        robotd --sim --stale-sensors-ms 100

  trunk height     0.1040 m  ->  0.0521 m          standing is ~0.116 m
  observation age  median 109.7 ms                 the bound is 80 ms
  move.limited_by  {}  on all 84 driving ticks

                                    robotctl health, read at that moment:
                                      robot     healthy
                                      loop      49.8 of 50.0 Hz · 530 ticks · 1 missed
                                      bus       ok
                                      imu       ready
```

**SAY**

> "One hundred milliseconds of delay in the sensor path. Nothing else changed — same build,
> same policy, same body, same fifty hertz.
>
> The duck went down. Trunk height a hundred and four millimetres to fifty-two.
>
> And read the right-hand column. Healthy. Forty-nine point eight of fifty hertz. One missed
> tick out of five hundred and thirty. Bus ok. IMU ready. The limit field — the field whose
> entire job is to tell you the safety layer refused something — **empty on every one of the
> eighty-four ticks it was driving.**
>
> The age was on the wire the whole time. A hundred and ten milliseconds, published, every
> tick, in the state stream, where anything could have read it.
>
> Nothing read it."

**SPEAKER NOTE — the honest disclosure, and say it here rather than in Q&A.** The demo this is
ported from had a different hook: *the robot succeeded and the contract failed* — it lifted the
cube on a 600 ms observation. **That hook does not exist on this duck, and I looked for it.** The
harness sweeps for it on every single run and prints the answer:

```
-- injected  60 ms -> age  65.9 ms  fell   travelled 0.512 m  trunk_z 0.0359
-- injected  80 ms -> age  88.6 ms  fell   travelled 0.297 m  trunk_z 0.047
-- calibration: no
```

> "I'll be straight with you about something. The version of this demo I ported had a better
> sentence: the robot *succeeded*, and the contract failed. I wanted that sentence. This duck
> won't give it to me — its balance gives out somewhere around fifty to sixty-five milliseconds
> of staleness, which is *below* the eighty the coast implies. So there is no delay where it
> walks and the contract is violated.
>
> I could have tuned the threshold until the sentence worked. I didn't, and the sweep that says
> so runs every time the demo runs, and it's in the bundle.
>
> What I got instead is, I think, worse in the way that matters: **it didn't succeed. It fell
> over. And every instrument still said it was fine.**"

*This is 35 s you cannot cut. It is the talk's own thesis applied to the talk's own demo, live,
and it buys you the right to everything in Act II.*

---

### 5:50 · Handoff into Act II

**SAY**

> "So we wrote the contract down, and three tests to pin it. They catch this.
>
> Look at who wrote the tests, and who wrote line four twenty-three."

*Straight into the existing 6:00 beat, unchanged: **"Same person. Same afternoon."***

---

## Where the repair goes

**Not in Act I.** The old structure spent Act I on the bug and the ladder on the response; keep
that. The repair belongs in **Act III — What survived (17:00 – 21:30)**, as a 90-second beat
before the cache material, because it is the one place in the talk where a fix *held*.

### 17:00 (new, 90 s) · The repair, and what a structure buys that a check does not

**SCREEN** — the patch stat, then the verdict pair, then the type.

```
$ python3 scripts/duck-demo/check-patch.py demo/freshness-patch.diff
candidate allowlist passed: duck-control/src/safety.rs
applied: +22 -1   one file

$ cargo test -p duck-control --lib
test result: ok. 83 passed; 0 failed

PROTECTED VERDICT: FAIL (seeded;   2 of 53 checks)
PROTECTED VERDICT: PASS (repaired; 57 checks; complete coverage matrix)
```

```
stale_100ms_turn, repaired:   stale_observation on 89 of 89 driving ticks
                              displacement 0.0003 m
```

**SAY**

> "Twenty-two lines, one file, enforced by an allowlist. Three tests go green. The verifier — the
> same verifier, one rule for both revisions, deriving from the traces whether a refusal was
> *required* — flips from fail to pass. Eighty-nine dispatches out of eighty-nine refused. The
> duck moved three tenths of a millimetre.
>
> **And I want to tell you why I don't think the check is the interesting part.**"

**SCREEN** — `duck-control/src/io.rs`, the field and its comment.

```rust
/// **Private, and an `Instant` rather than a number of nanoseconds, because
/// `Instant::now` is its only constructor: there is no way in safe Rust to make one in
/// the past.** A backend can keep a stamp, copy it, buffer it, hand it over late — it
/// cannot *date* a sample it did not take.
observed_at: Instant,
```

**SAY**

> "The timestamp on a sensor reading is a private `Instant`. Not a `u64` of nanoseconds — an
> `Instant`, whose only constructor is `now`.
>
> You cannot make one in the past. A driver can hold on to a stamp, copy it, hand it over late.
> It cannot *write* one. Which means the age the safety layer computes is the truth about that
> sample **even if the thing that handed it over was written to deceive me** — including the ones
> written after that comment.
>
> The `if` statement is a check. It works because I remembered to write it. **The `Instant` is a
> structure. It works whether or not anyone remembers anything.**
>
> That's the whole distinction this talk is trying to earn."

**And the honest ending of that beat — do not skip it:**

> "I'll give you the hole in my own argument, since that is apparently the format.
> `Sensors::default()` is still public, and it stamps `now`. So somebody *can* conjure a sample
> nothing ever observed and get a zero-millisecond age out of it. I know, because a reviewer
> wrote that test and it passes.
>
> The stamp is unforgeable. The freshness *claim* is not. Deleting `Default` is the move my own
> thesis demands, and it is a bigger diff than the rule was, and I have not done it. **That is
> the shape of every real system: the structural fix costs more than the check, which is why
> people write the check.**"

*90 s. Placing it here means Act III now reads: the robot fix that held → the build measurement
that held → the rung that held. Three instances of one shape.*

---

## Four consequential changes elsewhere

### 1. Act IV close (21:30) — the callback gets stronger, and it is now a quotation

The existing close says *"The policy holds no IO handle — it cannot command a motor."* That was
a description of our design. It is now **a quote from microduck's own source**, and
`robotd/src/control.rs`'s header doc says it in as many words:

> *"the safety layer's apply. It holds no IO handle — by construction it cannot command a
> [motor] directly."*

**SAY** (replacing the first clause of the existing close)

> "I didn't write that sentence. It was in the repo when I cloned it, in a header comment,
> justified by an incident. **The people who build robots for a living had already worked out
> that 'must not' and 'cannot' are different words.** All I did was apply the same move to time
> instead of to IO — and then leave the more expensive half of it undone."

### 2. Q&A #4 — "Is 250 ms real?" — answer replaced

**Was:** *"On the old harness it was illustrative. On a 50 Hz loop it is 12.5 control periods."*

**Now:**

> "There is no 250 ms any more. It's 80 ms, and it's four control periods: the coast's three
> plus the one the policy is trained on. The loop reports its own missed ticks, so I'm not
> taking anyone's word for the period. **And it turned out to be too lax** — this duck's balance
> gives out around fifty to sixty-five milliseconds of staleness, under *sustained* staleness,
> which is below the bound. That doesn't mean the coast constant is wrong; three consecutive
> stale ticks followed by a fresh one is a different load from a sensor path that is permanently
> behind. It means a bound derived from the coast is not a balance guarantee, and I'd rather say
> that than quietly move the number until the demo looked better."

### 3. Q&A #7 — "Is the seeded bug contrived?" — strengthened

> "`let _ = age;` is one token shorter than the version in the demo I ported from, and it is the
> canonical way to silence an unused parameter. Grep `let _ =` and you'll find it in seconds —
> but you have to already suspect it. Everything you'd *look* at says the rule is implemented:
> the limit variant, the wire name, the config field, the registry entry, three unit tests. That
> asymmetry is the point. **The artifacts of a rule are cheap. The rule is one comparison.**"

### 4. NEVER SAY — three additions

| ✗ | ✓ |
|---|---|
| "the duck walked with a stale observation" | **it did not — it fell. The success-with-broken-contract case does not exist on this duck** |
| "the repaired duck stands there calmly" | **it holds where it was put, on the floor — the injector is a start-time flag, so it never rises** |
| "the allowlist means a test can't be loosened" | **the tests live inside the allowlisted file. The protected matrix verifier catches that, not the allowlist** |

Keep every existing row. *"Complete coverage matrix"* is now literally what the duck verifier
prints (`57 checks; complete coverage matrix`) against a declared coverage file of nine required
properties — so that row's caution applies to the robosuite demo, not this one, and should be
annotated rather than deleted.

---

## Slide list delta

Slides 6–15 are unchanged. Slides 1–5 become:

```
 1  the duck, live — ctl health + drive
 2  microduck: 576 packages, 117k LOC, 50 Hz, Apache-2.0, not mine
 3  the timeline: 20 ms · 4 periods = 80 ms · dispatched at 109.7 ms
 4  cargo test: 80 passed, 3 failed — the three about freshness
 5  let _ = age;   (with the doc comment above it, full size)
 5b the fall: trunk 0.1040 -> 0.0521 beside "healthy / 50 Hz / bus ok / imu ready"
```

New slide in Act III, before the cache table:

```
16  observed_at: Instant   — "cannot date a sample it did not take"
    + the hole: Sensors::default() is still public
```

---

## Stage risk register for Act I

| risk | likelihood | mitigation |
|---|---|---|
| green-room arc fails | **real** — 0/8 for reviewers on a contended machine, 1/1 for me on a quiet one | run it 40 min early, on a machine with nothing else on it; kill every `robotd`/`body_server` and confirm `git status` clean first; if it fails, the bundle at `outputs/evidence-live/duck/20260924T134454Z` is already good and already re-verifies |
| `duck-sim drive` misbehaves on stage | low | the beat survives on `ctl health` alone |
| `cargo test` slow (cold target dir) | low | build the workspace in the green room; test time is 0.07 s once built |
| someone asks for the arc live | certain, from one person | `verify.py` against the pre-made bundle is under a second and is the same verifier — offer that, and say why the matrix is not live: 194 s, and the harness has no lock |
| asked why not robosuite | likely | it still passes 17/17 in 76.58 s; it is the fallback; the duck is here because 80 ms is arithmetic and the `Instant` is a structure |

---

## Budget

| | |
|---|---|
| Act I as written above | **~4 m 45 s** of the 6 min slot, incl. ~35 s of live machine |
| new Act III repair beat | 90 s |
| Act III material displaced | none — Act III was 4 m 30 s for three beats; it becomes 6 min. **Take the 90 s from Act II's forgery three, which is the one that survives compression.** |
| live commands on stage, total | `ctl health` ~1 s · `drive` 8 s · `cargo test` ~2 s = **~11 s** |
| green-room arc | 193.89 s measured, budget 5 min with setup |
