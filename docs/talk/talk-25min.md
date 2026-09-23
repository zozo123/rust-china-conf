# A Million Compiles. One Robot Hour.

**Full speaker script — 25-minute cut (fits the published 15:15–15:45 slot as 25+5).**
Technical subtitle: *Disposable Runners, Warm Cargo Factory.*
Mantra: **Burn the runner. Keep the proof. Spare the robot.**

Everything in quotes is meant to be said verbatim. [Brackets] are stage
directions. Timings are rehearsal budgets, not claimed runtimes.

---

## 0:00–3:00 — The robot-hour stakes

[Slide: title. Then the simulator view: one Panda arm, one cube on a table.]

> "Imagine your team has one hour booked on a robot tomorrow. Agents can
> produce many proposed changes overnight. Which change deserves that hour?
> Today the robot is simulated, so we can repeat failures cheaply. We will
> repair a Rust defect, destroy the machine that produced the patch, and
> validate the change on a fresh Linux runner using Incredibuild."

[Point at the cube.]

"This is the whole task: lift the cube. It is deliberately boring. A visible
job with a clear success condition — the simulator's own success check plus
an explicit height condition. What is *not* boring is everything around it:
who authorizes the arm to move, on what information, and who checked the
build that made that decision."

"One honesty note up front, and I will come back to it at the end: this is
software-in-the-loop. No physical controller, no physical robot. Catching
failures here is cheap. Hardware validation remains a separate downstream
stage — and that is exactly why we are doing this."

## 3:00–7:00 — Agent demand: candidates, failures, context

[Slide: agents → disposable runners → warm Cargo factory [IB] → retained
evidence → simulation → future HIL/robot gate. The final hardware stages are
labeled "not performed".]

"Overnight, your agents produce candidate changes. Each candidate needs the
same thing: a bounded work order, the failing evidence, a narrow edit scope
— and a machine to try it on. Our machine looks like this."

[Show the safety gate contract: stop first, timestamp validity, then the
250 ms freshness rule.]

"The supervisor gate is pure Rust. Three rules, in order: simulated
emergency stop beats everything. Future or invalid observation timestamps
are rejected. And a task action is permitted only when its observation is at
most 250 milliseconds old at the moment of dispatch. Two-fifty is an
illustrative demo policy — not a safe threshold for real hardware."

"Yesterday's build has a bug. The freshness check is missing. Watch what
that means."

[Show the seeded failure trace from runner A: observation age 600 ms,
decision: permit, four dispatches.]

"The observation stream was delayed by 600 milliseconds — real hold steps,
simulated time advancing, a genuinely old observation. The gate permitted
every pickup segment anyway. Nothing crashed. The cube even lifted. The only
thing that caught it is a protected assertion: *a stale episode must have
zero task-action dispatches.* This is why the verifier is protected and the
tests are not agent-editable."

## 7:00–10:00 — The build graph: distribution, cache, clean runners

[Slide: the Cargo dependency graph of the demo workspace; IB wrapper in the
compile stage only.]

"Every candidate needs another check. It does not need every dependency
compiled from scratch. The compile stage is the only accelerated stage:
tests and simulator checks always re-execute; their verdicts are never
restored from cache. Runner A starts with empty outputs and an empty cache
namespace. Runner B is a brand-new instance — same base image, fresh
filesystem — and reuses the compilation work the parent revision warmed."

[Name the honesty constraints explicitly.]

"We pin the toolchain, the lockfile, the target triple, flags, and output
paths. Dependency downloads are outside the timed section. If helpers are
not actually executing work, we do not show distribution. A cache hit rate
always names its denominator. And the speedup claim is a controlled
same-candidate comparison — the live broken-to-fixed sequence you are about
to see is workflow continuity, not the benchmark."

## 10:00–16:00 — The protected six-minute demonstration

[Switch to the runbook view: simulator left, restrained terminal right.
Follow docs/talk/runbook-6min.md. Key beats with lines:]

**0:00–0:30** — "Arm, cube, one job. The observation stream is about to lie
by 600 milliseconds."

**0:30–1:15** — Runner A cold build; failing acceptance case.
> "Runner A is a disposable sandbox: fresh worktree, empty outputs, empty
> cache namespace. There is the failure — permitted pickups on
> 600-millisecond-old eyes."

**1:15–2:05** — Agent attempt (capped ~45 s; fallback patch visible).
> "The agent gets the work order, the failing assertion, and a narrow edit
> scope — the gate source only. The protected tests, the threshold, the
> simulator: untouchable. The diff validator enforces that."

**2:05–2:30** — Show the diff; export context; destroy A.
> "Fourteen lines of intent. Export the patch, the base revision, the
> context packet. And now—" [destroy runner A]
> "The runner is gone. The change, the investigation, and the reusable
> compilation work survive."

**2:30–3:15** — Runner B: fresh instance, apply patch to the exact base,
warm build through IB, protected checks re-run.

**3:15–4:00** — Stale episode against the patched executable.
> "It refuses the stale request. Zero pickup dispatches — not because I say
> so; the protected verifier binds the trace to this exact binary's digest."

**4:00–4:40** — Fresh episode.
> "With current information, it completes the task." [Cube lifts.]

**4:40–5:20** — Evidence pack + measured build comparison.

**5:20–5:40** — Scope statement; **5:40–6:00** — buffer.

## 16:00–21:00 — Measurements and whole-loop economics

[Slide: benchmark table — same fixed source revision in every comparable
row; medians and ranges; run order and contention disclosed.]

"Same candidate, every row. Native cargo on a fresh runner as the baseline;
IB cold with helpers; IB warm from the parent revision only. Minimum five
runs per mode, cold caches reset each time, warm caches re-seeded from the
parent only. We report medians and ranges for compile/link, fresh tests,
simulator checks, and total validation — and separately runner startup,
checkout, artifact transfer, and agent response time, because that is where
the rest of the loop's time actually goes."

[If the 46 s → 13 s bands are not yet measured on the final environment:
say they are goals, not results. Never read unmeasured numbers as measured.]

"Wall-time saved is not CPU-hours, not cloud cost, not a faster model, and
not robot hours — unless you measure those things. We measure validation
latency: the time from 'candidate exists' to 'evidence says yes'."

## 21:00–25:00 — Evidence boundary, downstream gate, close

[Slide: the architecture diagram again, hardware stages labeled
"not performed".]

"What you saw: Rust contract checks, bridge checks, and simulated robot
scenarios, bound to the actual executable by digest. What you did not see:
hardware-in-the-loop, physical validation, a trained vision system — the
cube pose is simulator state; the camera feed is for you, not for the
controller. The 250-millisecond threshold is a demo policy. The simulator
lets us test this behavior before spending hardware time."

> "Every candidate needs another check. It does not need every dependency
> compiled from scratch. Incredibuild accelerates that Rust build loop. The
> simulator lets us test this behavior before spending hardware time.
> Burn the runner. Keep the proof. Spare the robot."

[Final slide: repo URL, evidence pack layout, the mantra.]

---

### 20-minute cut

2-minute opening (merge stakes + task), 3 minutes on agent loops, 3 minutes
on IB, the same protected 6-minute demo, 4 minutes of results, 2-minute
close. The published 30-minute slot supports 25+5 or 20+10 — not 25+10.
Both cuts preserve the demo untouched.
