# A Million Compiles. One Robot Hour.

**Full speaker script — 25-minute cut (fits the published 15:15–15:45 slot as 25+5).**
Technical subtitle: *Disposable Runners, Warm Cargo Factory.*
Mantra: **Burn the runner. Keep the proof. Spare the robot.**

Everything in quotes is meant to be said verbatim. [Brackets] are stage
directions. Timings are rehearsal budgets, not claimed runtimes.

Every number in this script comes from run `ec2-e2e-20260923-160725`, whose
evidence is committed under `docs/examples/`. If a number is not in that
evidence, it is not in this talk. See "Numbers you may say out loud" at the end.

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

[Open the stage visual — `docs/demo/index.html?play=seeded` — or show the
recorded runner-A trace directly.]

"The observation stream was delayed by 600 milliseconds. Not a paused clock:
real historical observations replayed while simulated time keeps advancing.
So the gate is handed genuinely old information. Four proposals — approach
at tick 12, descend at 22, grasp at 32, lift at 48. All four permitted."

[Let the audience look at the four green PERMIT lines.]

"And now the part I want you to sit with. The episode **succeeded.** The
cube came up to nine hundred ninety-eight millimetres. The simulator
reported success equals true. If you were reading a CI summary, this run is
green. Nothing crashed, nothing looked wrong, and the robot did the job —
on information that was already six hundred milliseconds stale."

"The only thing that caught it is a protected assertion: *a stale episode
must have zero task-action dispatches.* This one recorded four. That is why
the verifier is protected and the tests are not agent-editable. A build that
fails loudly is a gift. This one didn't."

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
always names its denominator."

"And here is the claim I am *not* going to make today. I have not measured a
speedup. I will show you what I measured, and I will tell you what it does
and does not mean. We will get to it."

### The infrastructure detour — worth three minutes

[Slide: the five walls.]

"A word about where this runs, because the story is the point.

I built this to run on ephemeral sandboxes from a provider called islo. That
was the design: a runner you rent for ninety seconds and throw away. And I
could not make it work. Not because the idea was wrong — because of limits,
one after another.

I asked for eight virtual CPUs — over quota. I asked for eight gigabytes of
memory — over quota. Forty gigabytes of disk — over quota. Incredibuild
wanted a ten-gigabyte helper cache; the sandbox had nine gigabytes free. I
moved to Debian 12, and the Incredibuild coordinator explicitly does not
support it. Five walls. I got past four of them. Then the account ran out of
credit, and the sandbox API told me so in one clean sentence:
*insufficient credit balance to create a sandbox.*

So the verified run you are about to see happened on a real Incredibuild
grid instead — a coordinator, an initiator, and two helpers on our own
Linux machines.

I am telling you this for a reason that matters more than the anecdote.
**None of the architecture changed.** Not one line. The runner lifecycle is
a fresh git worktree and a fresh Cargo target directory; the evidence pack
is a directory of files; the verifier binds its verdict to a SHA-256 digest
of the executable. That is why it moved between two completely different
kinds of infrastructure in an afternoon.

If your validation loop only works on one vendor's sandbox, you do not have
a disposable runner. You have a pet with a short life expectancy."

## 10:00–16:00 — The protected six-minute demonstration

[Switch to the runbook view: simulator left, restrained terminal right.
Follow docs/talk/runbook-6min.md. Key beats with lines:]

**0:00–0:30** — "Arm, cube, one job. The observation stream is about to lie
by 600 milliseconds."

**0:30–1:15** — Runner A cold build; failing acceptance case.
> "Runner A is a disposable runner: fresh worktree, empty outputs, empty
> cache namespace. Cold build, twenty-one point four seconds, distributed.
> And there is the failure — four permitted pickups on six-hundred-
> millisecond-old eyes, and a green-looking episode."

**1:15–2:05** — Agent attempt (capped ~45 s; fallback patch visible).
> "The agent gets the work order, the failing assertion, and a narrow edit
> scope — the gate source only. The protected tests, the threshold, the
> simulator: untouchable. The diff validator enforces that."

**2:05–2:30** — Show the diff; export context; destroy A.
> "Forty-nine lines of diff; four lines of actual intent — if the
> observation is older than the policy, reject it as stale perception.
> Export the patch, the base revision, the context packet. And now—"
> [destroy runner A]
> "The runner is gone. The change, the investigation, and the reusable
> compilation work survive."

**2:30–3:15** — Runner B: fresh instance, apply patch to the exact base,
warm build through IB, protected checks re-run.
> "Ten Rust tests — eight contract, two unit. On the seeded build, three of
> them failed. Here, ten of ten."

**3:15–4:00** — Stale episode against the patched executable.
> "Same six-hundred-millisecond observation. It refuses at tick twelve.
> Zero pickup dispatches — not because I say so; the protected verifier
> binds the trace to this exact binary's digest."

**4:00–4:40** — Fresh episode.
> "With current information, it permits all four segments and completes the
> task." [Cube lifts.]

**4:40–5:20** — Evidence pack + build comparison.
> "Seventeen episodes. Five cube placements against three observation ages,
> plus emergency stop and a protocol timeout. Ten lifted, five refused, one
> stopped, one timed out. Eighty-eight of eighty-eight protected checks
> passed, bound to digest f358e898."

**5:20–5:40** — Scope statement; **5:40–6:00** — buffer.

## 16:00–21:00 — Measurements, and the one I don't have

[Slide: the two build phases, and an empty benchmark table with the
methodology filled in.]

"Here is what I measured. Cold build on runner A: twenty-one thousand four
hundred twenty-six milliseconds. Warm build on runner B: twenty-one thousand
nine hundred forty-eight. Both phases ran through Incredibuild — that flag
is recorded in the evidence, and it is the thing I actually wanted to prove
on this run: the accelerated path was genuinely engaged on both sides, on a
real grid, with helpers.

Now read those two numbers again. The warm build was **five hundred
milliseconds slower** than the cold one."

[Pause. Let it land.]

"I could have left this slide out. I am showing it because a speedup number
is the easiest thing in this entire talk to fake, and the second easiest to
fool yourself with. These two builds are not a benchmark. Different runners,
different phases, one run each, a workspace small enough that distribution
overhead is plausibly larger than the work distributed. What they establish
is workflow continuity: the loop survived the runner being destroyed. That
is all they establish.

The benchmark I would need is specified and not yet run: the same fixed
source revision in every row, native Cargo as the baseline, IB cold with
helpers, IB warm from the parent revision only. Minimum five runs per mode,
cold caches reset each time, warm caches re-seeded from the parent only,
medians and ranges, run order and contention disclosed. Separately: runner
startup, checkout, artifact transfer, agent response time — because that is
where the rest of the loop's time actually goes.

When I have those numbers, they will be in the repo. Until then you have
heard me say twenty-one point four and twenty-one point nine, and you have
heard exactly what they mean."

"One more distinction, because it gets blurred constantly. Wall-time saved
is not CPU-hours, not cloud cost, not a faster model, and not robot hours —
unless you measure those things. The quantity worth optimising here is
validation latency: the time from 'candidate exists' to 'evidence says yes'."

## 21:00–25:00 — Evidence boundary, downstream gate, close

[Slide: the architecture diagram again, hardware stages labeled
"not performed".]

"What you saw: Rust contract checks, bridge checks, and simulated robot
scenarios, bound to the actual executable by digest. What you did not see:
hardware-in-the-loop, physical validation, a trained vision system — the
cube pose is simulator state; the camera feed is for you, not for the
controller. The 250-millisecond threshold is a demo policy, not a safe
hardware limit. And I have not measured a speedup."

"That list of things I did not do is not a disclaimer. It is the deliverable.
An evidence pack whose boundary you cannot see is not evidence — it is a
green checkmark with better production values. The seeded run in this demo
was green. That is the whole lesson."

> "Every candidate needs another check. It does not need every dependency
> compiled from scratch. Incredibuild accelerates that Rust build loop. The
> simulator lets us test this behavior before spending hardware time.
> Burn the runner. Keep the proof. Spare the robot."

[Final slide: repo URL, evidence pack layout, the mantra.]

---

## Numbers you may say out loud

All from `ec2-e2e-20260923-160725`, committed under `docs/examples/`.

| Claim | Value | Source |
| --- | --- | --- |
| Seeded build, stale episode | 4 dispatches permitted, ticks 12/22/32/48 | `ec2-runner-a/events-stale_600ms.jsonl` |
| Seeded episode outcome | `success=true`, cube 0.9985 m, 65 ticks | `ec2-runner-a/scenario-results.json` |
| Patched, stale episode | refused at tick 12, 0 dispatches, cube 0.8209 m | `ec2-runner-b/events-center-f600.jsonl` |
| Patched, fresh episode | 4 permits at ticks 0/11/21/37, cube 0.9978 m | `ec2-runner-b/events-center-f0.jsonl` |
| Episode totals | 17 runs: 10 lifted, 5 refused, 1 stopped, 1 timeout | `ec2-runner-b/scenario-results.json` |
| Rust tests, patched | 10 / 10 (8 contract + 2 unit) | contract.rs, lib.rs |
| Rust tests, seeded | 3 failures | contract.rs |
| Protected verifier | 88 / 88 | verifier output |
| Cold build | 21426 ms, `ib: true` | `ec2-runner-a/build-metrics.jsonl` |
| Warm build | 21948 ms, `ib: true` | `ec2-runner-b/build-metrics.jsonl` |
| Executable digest | `f358e898b4e4f0cf00c840de314c9e7c42cb246a4b97f1abffd3071fbecc8326` | `ec2-runner-b/manifest.json` |
| Simulator | robosuite 1.5.2 / MuJoCo 3.9.0, CPython 3.12.14 | `ec2-runner-b/manifest.json` |
| Incredibuild | Linux 4.31.0, coordinator + initiator + 2 helpers | grid config |

**Do not say:** any speedup factor, any cache hit rate without its
denominator, any hardware or HIL result, anything about robot-hours saved,
or that the islo sandbox path completed. It did not.

---

### 20-minute cut

2-minute opening (merge stakes + task), 3 minutes on agent loops, 2 minutes
on IB plus the infrastructure detour trimmed to its last paragraph, the same
protected 6-minute demo, 4 minutes of measurements, 2-minute close. The
published 30-minute slot supports 25+5 or 20+10 — not 25+10. Both cuts
preserve the demo untouched.

If you must cut further, cut the build graph section. Do not cut the
"warm was slower" slide or the scope statement; they are the two moments
that make the rest of it credible.
