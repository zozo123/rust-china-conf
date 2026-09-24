# A Million Compiles. One Robot Hour.

[简体中文](talk-25min.md) · [Runbook](runbook-6min.en.md) · [Slide outline](slides.en.md)

**25-minute speaker cues, including a six-minute demonstration, followed by five minutes of questions.**
The timings are rehearsal budgets. The title describes the motivation, not a measured conversion between builds and robot time.

## 0:00–2:00 · The robot succeeded

[Show the seeded replay. Keep the task and contract verdicts visible together.]

> “The robot lifted the cube. The simulator reported success.
> Would you trust the change?
>
> Look at the timestamps. Every motion segment used information that was
> 600 milliseconds old. Our policy allows at most 250.
>
> The task succeeded. The contract failed.”

[Pause on the two verdicts.]

“This is a simulated Panda arm in robosuite and MuJoCo. We use simulation to
make the mistake repeatable. Physical hardware validation is a later stage.
The question today is how a proposed code change earns that next stage.”

## 2:00–5:00 · What Rust decides

[Show the compact decision contract and the 250/251 ms boundary tests.]

“The controller proposes approach, descend, grasp and lift. Rust decides whether
each segment may start. Simulated stop takes precedence; future timestamps are
invalid; an observation older than the policy must be rejected.

The defect is deliberately small: the freshness check is missing. A successful
compilation does not establish this behavioral property. Neither does a
successful lift. The contract test asks the additional question.”

Explain observation age as simulation time minus capture time. The bridge
advances the simulated clock and supplies a historical observation; it does
not create age by sleeping while the simulator is paused.

“One important boundary: authorization is at segment dispatch. A segment can
contain multiple control steps. We are not demonstrating continuous physical
safety supervision. The threshold is an illustrative policy, not a hardware
safety limit.”

## 5:00–8:00 · Candidates need independent checks

[Show the work order, permitted edit path and protected tests.]

“The agent proposes an implementation change. It does not get to redefine what
passing means. Candidate patches can touch the gate implementation; acceptance
tests, simulator and verifier remain outside that patch.

We keep the failure trace, base revision and proposed diff. The verifier checks
that the required cases exist, that dispatches follow matching permits, and
that the exported binary has the recorded identity. Missing evidence fails.”

A digest identifies an artifact; it does not authenticate an entire execution
history or establish general correctness. The trusted orchestration and
protected checks remain part of the evidence boundary.

“The checked-in rehearsal uses a reviewed fallback patch. It can accept a patch
produced by an external agent. It does not itself call a language model.”
If using a live external agent today, show it explicitly and cap the attempt at
approximately 45 seconds; on timeout, switch to the reviewed fallback patch.
If using the fallback, say so before applying it.

## 8:00–11:00 · Disposable workspaces and Incredibuild

[Show the runner, compilation service and verification roles.]

“Every new candidate needs fresh validation. We also want temporary execution:
start clean, try the change, export the evidence and remove the workspace.

That creates a useful separation. The workspace is disposable. Eligible
compilation work can be reused by a build service. Test verdicts must be earned
again for this candidate.”

“Here, Cargo builds through Incredibuild's Linux wrapper with a rustc profile.
The intended benefit is compiler distribution and valid compilation reuse.
Tests and simulator cases execute separately. We must measure helper work and
cache reuse before claiming either occurred.”

“And this is a Rust conference, so Rust owns the proof path too. `swf-cli`
parses the Build History response and cache statistics. It rejects a run if
any IB sample has zero remote tasks, zero remote core time, ambiguous counters,
or—on the parent-warmed path—zero cache hits. Python is only the robosuite
adapter. It proposes motion; it cannot authorize it or certify the build.”

[Provider caption: EC2-backed workspaces today; islo provider planned.]

“This implementation uses detached Git worktrees and fresh Cargo output
directories on an existing Linux host. Removing a workspace does not destroy
the host or provide VM isolation. Islo is the intended future runner provider;
today's verified execution uses the EC2-backed grid.”

Keep the provider explanation under 30 seconds. Do not narrate quota failures.

## 11:00–17:00 · The six-minute demonstration

Use [the runbook](runbook-6min.en.md). The six visible beats:

1. Seeded cube lift: task completed, freshness contract failed.
2. Review the bounded patch.
3. Export the change and remove workspace A.
4. Build in B from the recorded base with fresh outputs.
5. Patched stale request: zero dispatches. Patched fresh request: cube lifts.
6. Inspect the evidence and executable identity.

For replay, say “recorded run.” For a new live run, show its own run ID and
values. A completed rehearsal matrix and two live selected cases must have
separate labels. Never display the old 88/88 as a new verifier result.

## 17:00–20:00 · What the measurements establish

[Show two observations, clearly labeled as historical, with the run ID visible.]

“Runner A's build phase took 21.426 seconds. B took 21.948.
B was 522 milliseconds slower. Both records indicate Incredibuild use.

This demonstrates the integration path across two workspaces. It does not
demonstrate a speedup, prove cache reuse, or show that helpers executed work.
The historical cold/warm labels were phase names, not controlled cache states.”

“The experiment harness now uses the same fixed candidate, native Cargo,
IB with an explicitly cleared per-user cache, and IB after that cache is
populated from the parent revision. It rotates run order and requires at least
five measurements per mode. The Rust verifier reports medians and ranges only
after every IB sample proves helper work and every warm sample proves cache
hits. Until an EC2 run produces that receipt, the performance claim remains
not measured.”

Separate compilation from provisioning, checkout, agent latency, tests,
simulation and export. A compilation improvement may or may not dominate the
complete candidate cycle. Do not translate it into cost or robot-hours saved
without measuring those quantities.

## 20:00–23:00 · What the evidence keeps

[Show one compact evidence receipt, then return to both robot verdicts.]

“The useful deliverable is the change plus inspectable evidence:
the source identity, candidate patch, actual executable, scenario results
and proposal/decision/dispatch trace.

The archived Linux run contains 17 episodes: 10 lifts, five stale rejections,
one simulated stop and one timeout. Its original verifier reported 88/88.
Our strengthened suite has a different scope and must produce its own result.”

The historical binary is not committed. The repository retains its digest and
logs; a new rehearsal exports its own executable for verification.

“The patched application refuses an old observation and still completes the
task with a current observation. That is the behavioral payoff.”

## 23:00–25:00 · Close

> “The robot lifting the cube was never enough.
>
> The candidate had to satisfy a contract, survive a fresh build, and produce
> evidence we could inspect.
>
> Our architecture separates temporary execution from reusable compilation.
> Incredibuild provides the build integration. Independent checks determine
> whether a candidate advances.”

[Final slide: repository, evidence link, one scope sentence.]

“Software-in-the-loop, segment-level authorization. No physical-robot result
and no measured speedup. Those are separate claims with separate experiments.”

## Questions and source notes

- **Why Rust?** A small typed decision API and deterministic clock inputs make
  the contract explicit and testable. Rust does not automatically enforce the
  correct freshness policy.
- **Why a robot?** Task completion and policy compliance can disagree visibly.
  The simulator provides a repeatable task without booking physical hardware.
- **What does IB accelerate?** Eligible compilation, not model reasoning,
  simulator physics or cached acceptance verdicts.
- **Why islo?** A future implementation of the disposable execution role.
  The demonstrated infrastructure is EC2-backed.
- **Why no speedup?** This run was a workflow demonstration with one observation
  per phase. The controlled comparison remains outstanding.

All historical numbers refer to `ec2-e2e-20260923-160725` in
[the archived evidence](../examples/README.en.md). Keep the main slide and the
run label visible whenever quoting them.

For a 20-minute cut, shorten the architecture and evidence discussions.
Preserve the behavioral comparison, demo, measured timings and scope.
