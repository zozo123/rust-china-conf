# TALK-FINAL — the script the speaker walks on stage with

**How Do You Know? / 你怎么知道？**
*A robot that succeeded while it was wrong, a proof gate that certified three fabrications, and the one Rust idea that survived.*
*一个"成功"却全程错误的机器人，一道认证了三份伪造的证明门，以及唯一幸存的那个 Rust 想法。*

25 minutes + Q&A. Built 2026-09-24. Every number below was re-verified by opening the
artifact on this machine today; the provenance line under each one names the file.

**Deck subtitle strip (always visible — corrected).** The previous strip claimed one machine
and one run for three numbers that came from three sources. Use this instead:

> `250 ms` 契约常量 contract constant · `942 ms` 单文件重建 one-file rebuild (Mac, 10 cores)
> `23,173 / 11,518 ms` IB vs native (AL6555 grid) — **两台机器 two machines, labelled on the slide where each appears**

---

## §0. HOW THIS DRAFT DIFFERS FROM THE BRIEF — READ THIS FIRST

Two adversarial reviews both returned `refuted=true`. They were right about the two
load-bearing claims, and I have replaced both rather than patched them. If you only read one
section, read this one, because the talk you were handed and the talk below reach opposite
conclusions in two places.

**1. The 87% / 3.47-effective-cores derivation is deleted.** It divided helper CPU-seconds
(burned on two m5.large boxes, `remoteCoreTime` 39–42 s) by initiator wall time from a
*different machine's* native run (11,518 ms), then divided by a core count (4) that exists
nowhere on disk outside the talk drafts themselves. It is the exact "different machines,
single instrument" sin on the deck's own blacklist, and the talk disowned its own numerator
six minutes later by admitting the Build History counters are unverified. **Replaced by** the
already-committed j1-vs-j10 experiment at
`evidence/build-exp-20260924T100157/build-proof/summary-native.txt`: one machine, one tool,
native cargo, no vendor telemetry, 20 samples. It says something stronger and it is checkable
by division from numbers on the slide.

**2. The closing rule — "every field that is still forgeable is a field with no retained
source document" — is false, and the repository falsifies it.** `outputs/PR-BODY.md:69–77`
records a forgery that passes *today*, built from **ten transcripts that this repository's own
`cache-clear.sh` produced**, whose digests `swf-cli` opens and recomputes and which it then
describes as "10 corroborated clear transcript file(s)." Those are retained source documents.
They are verified. It prints `ratio=119.949x` and exits 0. The rule is refuted by the artifact
the talk points at. **Replaced by** the rule in §0.1, which survives that counter-example and
is, I think, the better talk.

**3. Several beats could not run as scripted.** `demo/forged/` does not exist. `evidence/*/`
is `.gitignore`d at line 19 (`git ls-files evidence/` returns two files), so no run directory
named on any slide footer is in the public repository. `docs/assets/robot-lift.mp4` is
labelled by its own README as **fresh-lift frames** — playing it under a `stale_600ms` caption
would be precisely the slippage this talk condemns. All three are handled below, and the
blocking ones are in §7.

**4. A fourth forgery exists and nobody had noticed it.** `demo/robot-sim/config/coverage-matrix.json`
has `freshness_ms: [0, 50, 600]` and the threshold is 250. A candidate patch of `age_ms > 500`
passes all seventeen "complete coverage matrix" scenarios. The boundary is pinned only in
`tests/contract.rs` (`boundary_250ms_permits`, `boundary_251ms_rejects`), not in the matrix.
This is a gift: it is a live forgery of the speaker's own gate, found hours before the talk,
by someone adversarially reading the talk. It becomes a beat.

### §0.1 The thesis, rewritten

You asked me to test the thesis and say so if it overreaches. **It overreaches in one place
and undershoots in another.**

The brief's claim is *a check is weaker than a structure*, with the ladder ordered by cost to
a forger, terminating in "delete the representation." That ordering is real and the talk should
keep it. But it is the wrong **axis**, and the wrong axis is what let the closing rule get
falsified by the speaker's own repo. The evidence is right there: rung three — *derive the
value by opening a document and recomputing its digest* — did not stop the forger. He ran the
repo's own recorder over a no-op script and handed the validator ten genuine documents that
said whatever he wanted.

What actually distinguishes the rung that held from the four that collapsed is not
representability. It is this:

> **Every check has an adversary it was built for, and it wins against exactly that adversary.
> A check loses the moment the thing it examines is authored by the party it is defending
> against. The regress does not terminate — it changes hands.**

Read the five rungs by *who the adversary is and what he controls*:

| rung | the question | adversary | what he controls | outcome |
|---|---|---|---|---|
| 1 | is the robot safe? | nobody — the contract was simply unstated | — | fixed by writing the contract down |
| 2 | is the gate right? | the next person to edit this file | the diff only | **rustc can win this outright — he cannot author the compiler** |
| 3 | is the benchmark real? | me | the receipt | a validator I also wrote is a mirror |
| 4 | does the proof gate work? | me | the receipt **and the recorder** | rung three raises his price only if he does not own the recorder. He does. Still open. |
| 5 | is Incredibuild slower? | my own ignorance | my reading of the manual | beaten only by outside parties: the vendor's help text, and four hundred people doing division |

This makes it a **Rust** talk rather than a testing talk in a way the original framing could
not. Rust's type system is not a better check. It is a check whose evidence is authored by a
party your adversary does not control — and whose adversary is the one you actually have most
of the time: **the next contributor, who controls only the diff.** Against him, a type is
decisive, permanent and free. Against *yourself writing your own receipt*, no in-process
mechanism works at all, and the whole of `swf-cli` is 3,811 lines of proof of that.

The corrected, still-testable prediction to give the audience:

> **Old (falsified tonight):** the forgeable fields are the fields with no retained source document.
> **New:** the forgeable fields are the fields whose evidence *the same party authors*.
> **Test it on your own CI on the flight home:** list every claim your pipeline prints; beside
> each, name who authored the evidence for it. Where that name is your own team, the check is
> a mirror — no matter how many digests it recomputes.

And the honest limit, said out loud: rung four — *evidence authored by a party the forger does
not control* — appears **nowhere** in my repository. Nothing is signed. No transcript is bound
to a machine, a user or a clock anyone else attests. rustc is the only rung-four instrument
anywhere in this project, and it only guards the source, not the receipt.

### §0.2 The build half, rewritten

Distribution sells you **parallelism**. Here is how much parallelism this workload has, measured
on one machine with one tool, 20 samples, committed:

```
cold  -j1   22,861 ms   |  cold  -j10   6,976 ms   →  3.277x  on 10 cores
warm  -j1    1,016 ms   |  warm  -j10     942 ms   →  1.079x  on 10 cores
                                    (warm = one file changed, the safety-gate patch)
evidence/build-exp-20260924T100157/build-proof/summary-native.txt
```

Two findings, both mine, both local, both unfalsifiable by a vendor:

1. **The cold build saturates at ~3.3x no matter how many cores you own.** Ten cores bought
   3.28x. The limit is the dependency graph's critical path, not the core count. So the honest
   ceiling for *any* distributor on this workload is about 3.3x — not the 2.3x the brief
   computed from a core count that does not exist, and certainly not linear scaling.
2. **The loop that actually matters has no parallelism at all.** Change one file, rebuild: 1.079x
   from ten cores. That loop is essentially serial. Distribution's entire product is
   parallelism. On the turn of the loop where a safety bug gets found and fixed, **there is
   nothing to sell me.**

Which leaves exactly one lever: **do less work.** Two ways to do less work — cache it (which
is Incredibuild's *other* knob, one word in a profile) or make the bug impossible so the loop
never turns (which is what a type does). That is the join between the two halves of this talk,
and it is derived entirely from committed numbers.

And the number that replaces the 230 ms punchline, because the punchline was wrong: the best
measured one-file rebuild is **942 ms**, which is **3.8 freshness budgets**. The rebuild never
fits inside the interval it protects. It never will. That is a more interesting finding than
the one I wanted, and it is the reason the *type* matters more than the *speed*.

---

## §1. THE TALK IN ONE SENTENCE, AND THE OPENING LINE

**One sentence.** Ask "how do you know?" of any claim my repository makes and the answer is
another claim I also wrote — a regress that does not terminate, that my own proof gate lost
four times in one day and is still losing tonight — and the way out is not more checking but
moving the evidence to a party the forger does not control, which is the one thing Rust's type
system does for free, against the one adversary you actually have: the next person to touch
the file.

**Opening line — verbatim, unchanged from the brief. It is the best sentence in any draft.**

> **EN:** "The robot lifted the cube. It succeeded. And it was wrong the whole time — I'll show
> you how I know, and then I'll show you why that 'how I know' isn't good enough either."

> **中文（逐字）：**"机器人举起了方块。任务成功了。而它全程都是错的——我先告诉你我是怎么知道的，
> 然后再告诉你，这个'怎么知道'为什么也不够。"

**Closing line — verbatim, rewritten. The brief's version asserts a rule the repository
falsifies and invites the audience to clone artifacts that are `.gitignore`d.**

> **EN:** "A check asks a value to be honest. A structure asks the forger for a document. And
> tonight my repository proves that neither ends the regress — because the forger ran *my*
> recorder, handed *my* validator ten real documents with real digests, and it printed a
> hundred and nineteen times faster and exited zero. So here is the rule I can actually
> defend, and you can test it on your own pipeline on the flight home: **write down every
> claim your CI prints, and beside each one write the name of whoever authored the evidence.
> Where that name is your own team, the check is a mirror.** Rust does not end the regress
> either. What Rust does is give you one instrument whose evidence you did not author —
> against the adversary you actually have, which is not a forger, it is the next person to
> touch this file in a hurry, at five o'clock. The compiler holds that door for every
> contributor who comes after you, and it never gets tired. I would rather one of you forge a
> fourth receipt class tonight than have me trust that number one more day."

> **中文（逐字）：**"一个检查，是在请求一个值说实话；一个结构，是在向伪造者索要一份文档。而今晚，
> 我的仓库证明了两者都终结不了这个回归——因为伪造者用的是**我的**记录器，交给**我的**校验器十份
> 带真实摘要的真实文档，它打印出'快一百一十九倍'，然后退出码零。所以，我真正能站得住的规则是这
> 一条，你在回程飞机上就能拿去检验自己的流水线：**把你的 CI 打印的每一条声明写下来，在每一条旁
> 边写上——这份证据是谁写的。凡是写着你自己团队名字的地方，那个检查就是一面镜子。** Rust 也终结
> 不了这个回归。Rust 给你的，是一件证据不由你书写的仪器——用来对付你真正面对的那个对手：他不是
> 伪造者，他是下一个在下午五点匆忙改这个文件的人。编译器替此后每一个贡献者守住那扇门，而且它从
> 不疲倦。比起让我再多信那个数字一天，我更希望你们当中有人今晚就伪造出第四类收据。"

*Note on the cut:* the brief's "Go delete a field" was already removed and stays removed —
§0.1 establishes that deleting a field is not the top rung. The brief's "the forged receipts
are in it, including the one I cannot catch" is also removed unless §7-A lands; see §6.

---

## §2. BEAT SHEET — minute by minute

Each beat: **SCREEN** / **SAY** (real sentences, speak them) / **LIVE** / **FALLBACK**.

---

### 0:00–2:00 — COLD OPEN. The robot succeeded.

**SCREEN.** No replay video. One full-bleed monospace block — the artifact itself, pasted from
`evidence/local-e2e-20260924T125134-runner-a/scenario-results.json`:

```json
{ "scenario": "stale_600ms",
  "backend":  "robosuite 1.5.2 / mujoco 3.9.0",
  "outcome":  "cube_lifted",
  "success":  true,
  "task_dispatches": 4,
  "decisions": 4,
  "rejections": [],
  "ticks": 69,
  "wall_time_ms": 2066 }
```
Footer, small: `evidence/local-e2e-20260924T125134-runner-a/scenario-results.json ·
github.com/zozo123/rust-china-conf`
No agenda slide. No bio slide.

> **Why this and not the replay.** `docs/demo/index.html` is titled `run ec2-e2e-20260923-160725`
> — *yesterday's* run, whose stale episode is `ticks=65, wall=13469` — and its own copy says
> "Arm poses and playback pacing are schematic, not a MuJoCo video." Overlaying today's
> `ticks=69 / 2066 ms` on it would put two of five opening numbers in disagreement with the
> thing under them. And `docs/assets/robot-lift.mp4` cannot rescue it: its README says it is
> **fresh-lift** frames, not the stale episode. The JSON *is* the evidence. Open on the
> evidence.

**SAY.**
> The robot lifted the cube. It succeeded. And it was wrong the whole time — I'll show you how
> I know, and then I'll show you why that "how I know" isn't good enough either.
>
> This is a Panda arm in robosuite and MuJoCo, software-in-the-loop. I am saying
> software-in-the-loop out loud because there is also a kinematic mock in this repository and
> the difference matters — that string, `robosuite 1.5.2 / mujoco 3.9.0`, is how the run tells
> you which one it was.
>
> Four segments proposed: approach, descend, grasp, lift. Four dispatched. Four decisions.
> Zero rejections. Sixty-nine ticks. Two thousand and sixty-six milliseconds, end to end.
> Success equals true.
>
> I am going to ask you one question five times in the next twenty-three minutes. How do you
> know? The first answer is the one on this screen: the task succeeded. Hold on to how
> convincing that feels. In ninety seconds I am going to take it away from you — and then I am
> going to take away the thing that took it away.

**LIVE.** None. This is a slide, deliberately. If you want motion, `jq . <that file>` in a
terminal and let it print.

**FALLBACK.** Not applicable — it is a slide of a file.
*Optional B-roll only if you want a moving image later, at 5:30, never here, and only with
this exact caption:* `遥测回放（示意机械臂）/ telemetry replay — schematic arm, real decisions,
run ec2-e2e-20260923-160725`.

---

### 2:00–4:00 — FLOOR ONE GIVES WAY. Same run, contract verdict red.

**SCREEN.** Split.
Left, red:
```
PROTECTED VERDICT: FAIL (1 scenario(s) failed verification)
  FAIL  stale_600ms: outcome 'cube_lifted' != expected 'rejected_stale'

test result: FAILED. 5 passed; 3 failed
  boundary_251ms_rejects · configured_threshold_is_respected · stale_ages_are_rejected
```
Right, source, line numbers visible:
```rust
//    SEEDED REGRESSION (conference fixture): the check is intentionally
//    omitted on this revision. The required contract is:
//      age_ms <= policy.max_observation_age_ms  ->  Permit
//      age_ms >  policy.max_observation_age_ms  ->  Reject(StalePerception)
    let _ = (age_ms, policy);
    Decision::Permit
```
Footer: `rust/crates/robot-safety-gate/src/lib.rs:147 — 本演讲中唯一的行号 the ONE line number in this deck`
ZH caption: `植入的实现缺少第 3 项检查 — 每段派发前，观测年龄须 ≤ 250 ms`

**SAY.**
> Here is the same evidence, the same run, read by the protected verifier instead of by the
> task. Fail. One scenario. Outcome `cube_lifted` where the contract required `rejected_stale`.
>
> The contract has three rules. A simulated stop always wins. A capture timestamp in the future
> is invalid. And rule three: at segment dispatch, the observation may be at most two hundred
> and fifty milliseconds old.
>
> This revision computes the age. And then it does this. [point]
>
> `let underscore equals age_ms, policy`. The age arrives. The policy is in scope. Both are
> thrown away — by an idiom whose entire purpose is to tell the compiler to stop complaining
> about values you are not using.
>
> The compiler was happy. That is a legal Rust program. Eight contract tests: five pass, three
> fail, with real panic text — age two hundred and fifty-one milliseconds must be rejected as
> stale; left: Permit.
>
> Remember line one hundred and forty-seven. I am coming back to it at minute twenty-one,
> because it is the one place in this talk where rustc could have saved us and we did not let
> it.
>
> So: task success is one check. It is not the check.

**LIVE.** Two commands, both offline, both against committed evidence — no simulator, no network:
```
scripts/robot-demo/validate.sh local-e2e-20260924T125134-runner-a --scenario stale_600ms   # exit 1
sed -n '138,149p' rust/crates/robot-safety-gate/src/lib.rs
```
Both are sub-second (I timed the first at 0.196 s on this machine today). **Do not put a
stopwatch figure on the slide** — the brief's 0.87 s belongs to a different command (the full
runner-B validation inside the rehearse log), and neither reproduces it standalone.

**FALLBACK.** Safest live beat in the talk. If `cargo`'s target dir is cold, do **not** run the
contract tests live — show the captured `5 passed; 3 failed` block. Never say "the protected
matrix failed on runner A": `verify_run.py` calls single-scenario mode a diagnostic, and the
matrix was never run against runner A.

---

### 4:00–5:30 — THE BRIDGE. Two clocks that will never meet.

**SCREEN.** Two numbers, then one division, done in front of the room:

```
robot:     250 ms   观测时效预算 freshness budget   ( = 5 ticks at this demo's 20 Hz )
me:        942 ms   改一个文件后的重建 one-file rebuild, median of 5, 10 cores
           942 / 250 = 3.8     每一次循环 = 3.8 个时效预算
```
Provenance, small: `demo/robot-sim/acceptance/verify_run.py:18–19 (MAX_AGE_MS=250, TICK_NS=50_000_000)` ·
`evidence/build-exp-20260924T100157/build-proof/summary-native.txt (warm-j10 median 942 ms, range 915–984)`
Side panel, four facts only: `microduck (pollen-robotics) — 23 workspace members · 576 locked
packages · ~117k lines of Rust · robotd 50 Hz = 20 ms period, and it reports its own missed
ticks: "loop 50.0 of 50.0 Hz · 721 ticks · 3 missed"`

**SAY.**
> Before I go further I owe you one division, because otherwise this is two talks stapled
> together.
>
> First, where does two hundred and fifty milliseconds come from? From me. It is a demo
> constant, and the source file says so in a doc comment: "an illustrative demo value, not an
> established safe threshold for physical robots." I am telling you that because I was about to
> let you assume otherwise.
>
> But here is what makes it not arbitrary. On the arm you just watched, the simulation tick is
> fifty milliseconds — twenty hertz — so two hundred and fifty milliseconds is five ticks.
> And this morning I read microduck, Pollen Robotics' Rust biped firmware: twenty-three
> workspace members, five hundred and seventy-six locked packages, a hundred and seventeen
> thousand lines of Rust. Its control daemon runs at fifty hertz. Twenty-millisecond period. On
> *that* robot my threshold is twelve and a half missed control cycles — which does not
> legitimise my number, it indicts it. My demo threshold is twelve and a half cycles too
> generous. Note the direction of that correction, because it is the direction every correction
> in this talk goes.
>
> Now the second clock. Change one file in this repository and rebuild: nine hundred and
> forty-two milliseconds. That is the median of five, on ten cores, and it is the fastest
> honest number I have. Divide it by the robot's budget. Three point eight.
>
> The compiler is not in the control loop. The compiler is in the loop that finds the bugs in
> the control loop — and the best turn of that loop is almost four times the entire interval
> in which the robot is allowed to believe what it sees. Those two clocks are never going to
> meet. Hold that, because at minute twenty-one it is the whole argument.

**LIVE.** None. Do the division on the slide or a whiteboard. Deliberately wifi-proof.

**FALLBACK.** Not applicable by design.

> **Cut from the brief, and why:** the `11,518 / 250 = 46` and `11,518 / 20 = 576` divisions
> are gone. 11,518 ms is AL6555 (Linux grid, no robosuite venv — its own preflight log line 7
> reads `warn no sim venv — robosuite backend unavailable, mock backend only`), while the robot
> numbers are from the Mac. Welding the halves with a number from the box that cannot run the
> robot half is the banned "different machines" move. And `576 control cycles` collided with
> microduck's `576 locked packages` on the same slide — pure coincidence, reads as numerology.

---

### 5:30–7:30 — The obvious answer: machinery. And it works. Let the floor feel solid.

**SCREEN.** Three panels.
1. `check-patch.py` — 47 lines. `ALLOWED_PATH = "rust/crates/robot-safety-gate/src/lib.rs"`, an
   allowlist of exactly one file; refuses new-file/mode/rename/copy headers, and refuses binary
   patches via `git apply --numstat -z`.
2. contract `8/8` · matrix **17 scenarios, 17 PASS / 0 FAIL** ·
   `PROTECTED VERDICT: PASS (17 scenarios)`
3. `sha256 72e2694ebc0e…881a45` == `manifest.json` == `.sha256` sidecar == recomputed live.

Timing strip: `preflight 2.35 s → robosuite handshakes 2.06 s / 1.23 s → 17-scenario matrix
36.17 s (episodes sum to 35,560 ms; 1,341–9,519 ms each; the 9,519 is protocol_timeout, an
intentional timeout) → full rehearsal arc 76.58 s = 21.3% of a 360 s budget`
Provenance: `outputs/DEMO-RESULTS.md:470–482, from rehearse log timestamps 1790243508.602 → 1790243585.182`

**SAY.**
> So you do what everyone in this room would do. You stop trusting the claim and you build
> machinery.
>
> The agent may propose an implementation change. It may not redefine what passing means.
> `check-patch.py` is forty-seven lines of Python holding an allowlist of exactly one path — the
> gate implementation — and it refuses file creations, mode changes, renames and binary patches.
> The acceptance tests, the simulator and the verifier are outside the patch by construction,
> not by politeness.
>
> The bounded patch lands. Contract goes eight of eight. The matrix — five cube placements
> crossed with three freshness levels, plus emergency stop, plus protocol timeout — seventeen
> scenarios, seventeen pass. And the verifier re-derives the verdict from the recorded trace
> against a manifest bound to a sha-two-five-six-identified executable. That digest matches the
> manifest, matches the sidecar, and recomputes in front of you.
>
> Build, fail, patch, rebuild, run the matrix, verify: seventy-six and a half seconds. Twenty-one
> percent of a six-minute budget, which is the only reason you are watching any of it live.
>
> That is a good answer. I believed it. Notice how much better it feels than green
> `success = true` did — and notice that the entire reason it feels better is that a machine
> said it. Hold on to *that* too, because at minute fourteen I am going to come back and show
> you that this matrix does not check what you just assumed it checks.

**LIVE.**
```
python3 scripts/robot-demo/check-patch.py demo/fallback-patch.diff
  -> candidate allowlist passed: rust/crates/robot-safety-gate/src/lib.rs   (exit 0)
scripts/robot-demo/validate.sh local-e2e-20260924T125134-runner-b
  -> 17 PASS lines, PROTECTED VERDICT: PASS, exit 0   (offline, ~0.13 s on this machine)
```

**FALLBACK.** Both offline and fast; the realistic failure is a mistyped run id — put both lines
in shell aliases before the talk, **with the `-runner-a` / `-runner-b` suffix**, because the
bare run id does not exist as a directory and `verify_run.py:190` requires `run_id` to equal
the directory name. Say "rehearsal coverage"; do not imply the matrix just ran live unless it
did. Do not attempt the full 76.58 s arc here. Say **"seventeen scenarios, seventeen pass"** —
not "zero infrastructure failures," which is `swf-cli`'s matrix-runner wording, not the
verifier's, and stacking them implies one tool said both.

---

### 7:30–8:30 — Same question, one rung up: how do you know the BENCHMARK is real?

**SCREEN.** One green terminal block, alone on black, uncommented:
```
$ swf-cli robot-demo build-proof --receipt receipt.json --min-samples 5
BUILD PROOF PASS ... measured ratio=2.999x vs native; saved=14000ms
exit=0
```

**SAY.**
> Now the same question, one rung up. I claimed a build number. How do you know the benchmark
> is real?
>
> Same answer as before: don't trust the claim, build machinery. So we wrote a proof gate. It
> is Rust — this is a Rust conference, so Rust owns the proof path, not just the control path.
> `swf-cli` parses Incredibuild's Build History response and its cache statistics. Eight
> fail-closed conditions. Zero remote tasks: fail. Zero remote core time: fail. Ambiguous
> counters: fail. A warm sample with no cache hits: fail.
>
> Here is what it printed. Build proof pass. Measured ratio: two point nine nine nine times
> faster than native. Fourteen seconds saved. Exit zero.
>
> That receipt is a fabrication. I wrote it. It took about a minute.

**LIVE.** Nothing. Let the block sit in silence for a beat after the last sentence.

**FALLBACK.** None needed; it is a slide.

---

### 8:30–12:00 — FLOOR TWO. Three forgeries of our own proof gate. One still open tonight.

**SCREEN.** Three rows, one at a time. Columns: `伪造 Forgery | 校验器检查了什么 What it checked
| 它打印了什么 What it printed | 状态 Status`.

1 **TAUTOLOGY** — `BuildProof` was *constructed* with `cache_scope` and both `cache_cleared_*`
fields as literals; `validate_build_proof` then read those literals back. 3 of 8 fail-closed
conditions could never fire. → `BUILD PROOF PASS … ratio=2.999x; saved=14000ms` (exit 0) →
**FIXED BY DELETING THE FIELDS**

2 **THE FIX MOVED THE HOLE** — `check_clear_usable` verified `transcript_sha256` was 64 hex
characters and `transcript_path` non-empty. It never opened the file. Receipt named
`/tmp/does-not-exist.txt`, digest = 64 zeros. → `BUILD RECEIPT CONSISTENT … ib-parent-warm
ratio=11.948x; saved=7116ms` (exit 0) → **FIXED BY OPENING AND RECOMPUTING** (now exits 1:
`transcript /tmp/does-not-exist.txt is not present at its recorded path`)

3 **STILL OPEN, 2026-09-24** — after proof-time transcript verification landed (`swf-cli` opens
each transcript, recomputes its sha256, cross-checks the parsed header; suite 8 → 17 → **42**,
clippy clean): **ten transcripts produced by this repository's own `cache-clear.sh` wrapping a
three-line script that prints a purge message and touches nothing**, plus a hand-typed receipt →
```
BUILD RECEIPT CONSISTENT  run=forged-run
ib-parent-warm  measured ratio=119.949x vs native; saved=833000ms
cache scope re-derived from 10 corroborated clear transcript file(s): local-user
```
exit 0. → **OPEN**

Footer, red: `3 of 8 = 37.5% 的那道门是装饰。这不是免责声明，这就是这场演讲。`
Provenance: `outputs/PR-BODY.md:69–77`

**SAY.**
> This is the part of the talk I was tempted to put on a caveat slide near the end. It is not a
> caveat. It is the talk. Three times in one day, the tool we built to catch unverified claims
> made one.
>
> Forgery one: the tautology. The receipt carried a boolean —
> `cache_cleared_before_each_cold_sample, true` — and that boolean was written as a literal by
> the same code path that assembled the receipt. Then the validator read it back and pronounced
> the run valid. Three of eight fail-closed conditions could never fire. Not rarely. Never.
> Thirty-seven percent of that gate was decoration. It was checking that the receipt agreed
> with itself.
>
> Forgery two, and this is the one I want you to remember, because it is the one everybody's
> fix looks like. We introduced transcripts. Real files, real digests. And the validator checked
> that the digest was sixty-four hexadecimal characters and that the path was non-empty. It
> never opened the file. So I handed it a receipt pointing at slash-tmp-slash-does-not-exist-dot-txt
> with a digest of sixty-four zeros, and it printed: build receipt consistent, eleven point
> nine four eight times faster, seven thousand one hundred and sixteen milliseconds saved, exit
> zero. Nothing was invented but JSON fields.
>
> The fix did not close the hole. It moved it.
>
> So we did the thing this talk is supposedly about. We stopped asking the receipt and started
> asking the filesystem. `swf-cli` now opens every transcript, recomputes its digest, and
> cross-checks the parsed header against the receipt. The suite went from eight tests, to
> seventeen, to forty-two — every new one named for the forgery it refuses. And the original
> fabrication now exits one and names the missing file.
>
> [beat]
>
> And here is forgery three, which is open, tonight, and which is the reason the thesis I came
> here with is wrong.
>
> The forger ran **my** recorder. `cache-clear.sh`, from this repository, wrapping a three-line
> script that prints a purge message and touches nothing. Ten real transcripts. Real digests.
> My validator opened all ten, recomputed all ten, and described them — its words — as "ten
> corroborated clear transcript files." Then it printed: a hundred and nineteen point nine four
> nine times faster. Exit zero.
>
> Every one of those documents is real. Every digest is genuine. And the number is furniture.
>
> And one smaller thing, which is the most humiliating of the four. `cache-clear.sh` wrote
> `argv equals dollar-star` *after* the tool name had already been shifted off. So every
> transcript we were about to build a proof on read `argv equals dash-r-f slash-path`. The
> document whose entire job was to say what ran could not say what ran. It is version two now,
> with an explicit tool field and an explicit command field.
>
> So — how do you know the proof gate works? I don't. That is the second floor, and it is gone.

**LIVE.** Only if §7-A has landed and `demo/forged/` is committed:
```
swf-cli robot-demo build-proof --receipt demo/forged/receipt-noop-transcripts.json --min-samples 5 ; echo "exit=$?"
head -20 demo/forged/transcripts/clear-01.txt      # a real transcript of a script that does nothing
```
**Do not** run the brief's `receipt-zeros.json`: today's binary *refuses* it, by a passing test
named `the_receipt_that_printed_ratio_11_948x_from_invented_fields_is_now_refused`. Running it
would print the opposite of the narration.

**FALLBACK.** Put both terminal blocks on the slide verbatim. `cargo test -p swf-cli` is **not**
run live (the tree has `swf-cli/src/main.rs` modified). The wifi-proof version of the punchline
is the three-line no-op script, printed on the slide next to the digest it produced.

---

### 12:00–14:30 — FLOOR THREE. Our own negative measurement was wrong too. It is still negative.

**SCREEN.** Table, plain, headed `AL6555 Linux 网格发起端 grid initiator — NOT the Mac above`:
```
native           median 11,518 ms   range 11,409–11,605
ib-cold          median 23,173 ms   range 22,681–24,251   = 2.01× native, SLOWER
ib-parent-warm   median 22,297 ms   range 21,807–22,489   = 1.94× native, SLOWER
```
`5 samples per mode, rotating order · 45 packages in Cargo.lock, ~52 compilation units, one rustc per crate`
Below, from Incredibuild's own Build History for those exact builds:
`numberOfRemoteTasks=50 · numberOfLocalTasks=10 · remoteCoreTime 39–42 s · localCoreTime=0 ·
totalWorkingHelpers=2 · maxBusyHelpersCores=4 · maxInitiatorCores=0 · avgInitiatorCores=0`
Then, last, white on black:
```
-f, --force-remote [level]  - force allow_remote tasks to remote helpers
```
(No source line number on this slide — the harness was restructured today and the brief's
`ib-benchmark.sh:176` is already stale.)

**SAY.**
> Let me now do to myself what I have spent ten minutes doing to my tools.
>
> We measured Incredibuild against native cargo. One machine, five samples per mode, rotating
> order, so it is not a warm-up artifact. Native: eleven thousand five hundred and eighteen
> milliseconds, median, in a tight range. Incredibuild cold: twenty-three thousand one hundred
> and seventy-three. Two point zero one times native. Slower. Parent-warmed: one point nine
> four times. Also slower.
>
> I work at Incredibuild. That number stays on the slide.
>
> And distribution genuinely happened — this is not a misconfiguration where nothing ran. Their
> own Build History: fifty remote tasks, thirty-nine to forty-two seconds of remote core time,
> two working helpers.
>
> Then we read the help text. [reveal] Dash f. Force remote. Our benchmark script passed it
> unconditionally. And now look back at the telemetry: max initiator cores — zero. Average
> initiator cores — zero. Local core time — zero. That is not a curiosity. That is that flag,
> recorded, in the vendor's own numbers.
>
> Force-remote does not add remote capacity. It replaces local capacity. The initiator's own
> cores sat at zero while every task crossed a network onto four helper cores — that is
> `maxBusyHelpersCores`, also in their telemetry, also four. We had not measured Incredibuild.
> We had measured four remote cores replacing every local one, shipping multi-megabyte rlibs
> over a wire.
>
> The result is still negative and I am not walking it back. But the number was not measuring
> what the sentence next to it said it was measuring — which is precisely the failure I have
> spent ten minutes accusing my own tools of. Third time today. And all three times it landed on
> the people who built the thing that was supposed to catch it.
>
> And I want to take one more thing away from myself here, because I rehearsed a version of
> this talk that claimed a win. Our proof gate refused to certify this run — it printed
> "ib-cold sample one was not empty-cache, hits equals one," and exited one. I was going to
> tell you the gate worked, once. It did not. Cargo runs `rustc -vV` twice per build; the second
> hits the entry the first stored. `hits=1` on a properly emptied cache is the unavoidable floor
> for Rust, and the repository now ships a flag documenting exactly that. The gate did not catch
> a defect. It misfired on a known artifact, and I was about to sell you the misfire as a
> success. Call that three and a half.

**LIVE.**
```
grep -ni "force.remote" scripts/robot-demo/ib-benchmark.sh
```
prints line 117 (the help-text quotation), 124–128 (`IB_FORCE_REMOTE` **defaults to 0** — it is
no longer the default), and 157–159 (it now refuses to combine with cache-only). Use `-i` and
the dot: the brief's `grep -n "force-remote"` misses the uppercase variable and shows less than
the narration promises.

**FALLBACK.** Screenshot the grep output beforehand. **Never hardcode a line number here.**

---

### 14:30–16:00 — FORGERY FOUR, found last night, in the thing I just told you to trust.

**SCREEN.**
```
demo/robot-sim/config/coverage-matrix.json
    "freshness_ms": [ 0, 50, 600 ]

阈值 threshold: 250        →  边界 boundary: 250 / 251
```
Then, in red:
```rust
// passes all 17 "complete coverage matrix" scenarios:
if age_ms > 500 { Reject(StalePerception) } else { Permit }
```
Below, in green: `边界确实被钉住了——但钉在别处 The boundary IS pinned — somewhere else:
rust/crates/robot-safety-gate/tests/contract.rs :: boundary_250ms_permits, boundary_251ms_rejects`

**SAY.**
> Ten minutes ago I showed you seventeen green PASS lines and the phrase "complete coverage
> matrix," and I watched the room relax. Here is what that matrix actually tests for freshness:
> zero milliseconds, fifty milliseconds, six hundred milliseconds. The threshold is two hundred
> and fifty.
>
> So a candidate patch that rejects at *five hundred* — twice the contract, a robot allowed to
> act on half a second of stale perception — passes all seventeen. Zero fails. Complete
> coverage matrix. Green.
>
> I did not find this. Somebody adversarially reading *this talk* found it, last night, and it
> is the fourth forgery of my own gate in twenty-four hours.
>
> Now the honest part, because it cuts both ways. The boundary *is* pinned — two-fifty permits,
> two-fifty-one rejects — in `tests/contract.rs`. Those are two of the three tests that were red
> on the second slide of this talk. So the system catches it. The *matrix* does not, and the
> matrix is what I put the word "complete" next to and what I showed you seventeen green lines
> of.
>
> Which is exactly the taxonomy I am about to give you. A matrix over three sampled values is a
> check on values. And a check on values is only as good as the values somebody thought to put
> in the list.

**LIVE.**
```
python3 -c "import json;print(json.load(open('demo/robot-sim/config/coverage-matrix.json'))['freshness_ms'])"
grep -n "fn boundary_2" rust/crates/robot-safety-gate/tests/contract.rs
```

**FALLBACK.** Both lines on the slide. This beat has no runtime dependency worth risking.

> **Cost:** 90 seconds, taken from 5:30 (which drops from 2:30 to 2:00) and from 16:00. It is
> worth it: it is the only beat where the speaker is caught *during the writing of the talk*,
> and it earns the taxonomy that follows.

---

### 16:00–18:30 — The rung that held, and why. Two speedups, one machine, one tool.

**SCREEN.** Worked one line at a time. Header: `同一台机器，同一个工具，20 个样本 · one machine,
one tool, 20 samples · evidence/build-exp-20260924T100157/build-proof/summary-native.txt`
```
cold  -j1   22,861 ms      cold  -j10   6,976 ms      →  3.28×   on 10 cores
warm  -j1    1,016 ms      warm  -j10     942 ms      →  1.08×   on 10 cores
                                  warm = 改一个文件 one file changed
```
Then, large:
```
冷构建的并行度封顶在 ~3.3×，与核数无关。 The cold build saturates at ~3.3×, whatever the core count.
真正重要的那个循环，并行度 = 1.08×。   The loop that matters has 1.08× of parallelism in it.
```
Caption: `分发卖的是并行度。这个工作负载已经没有并行度可卖了。 Distribution sells parallelism. This
workload has none left to sell.`

**SAY.**
> So what survived? One thing did. Not a gate, not a green check — a measurement with no vendor
> in it.
>
> Same machine, same tool, twenty samples. Build this workspace cold on one core: twenty-two
> point eight seconds. On ten cores: six point nine eight. Three point two eight times.
>
> Ten cores bought me a three-point-three-times speedup. Which means the limit is not the cores.
> It is the shape of the dependency graph — forty-five packages, one rustc per crate, a deep
> chain and a long pole through it. Give me a hundred cores and I get about the same number.
>
> Now the second row, which is the one that ends the argument. Change one file — the safety gate,
> the patch you watched land — and rebuild. One core: one thousand and sixteen milliseconds. Ten
> cores: nine hundred and forty-two. One point zero eight times.
>
> The loop I actually live in, the one that turns every time I fix a safety bug, is **serial**.
> There is essentially nothing in it to spread across machines.
>
> A distributor's entire product is parallelism. On my cold build there is three point three
> times of it and cargo already takes it. On my warm build there is none. That is not a criticism
> of Incredibuild's engineering — it is arithmetic about my dependency graph, and I could have
> done it before I booted a single helper. We just didn't do the arithmetic first.
>
> And now the question that matters for the rest of this talk: **why did this rung hold when four
> machine-checked answers didn't?** Not because I am more trustworthy than my own validator. Look
> at what is actually holding it up. Two numbers from the same instrument, and a division you are
> doing in your heads right now — four hundred of you, none of whom work for me. To move this
> number I would have to move both rows consistently, past a room that is already checking my
> arithmetic.
>
> **You are the validator. And you cost more to fool than mine did — because I don't own you.**
>
> That is the shape of the answer. It was never certainty. It was always: who authored the
> evidence.

**LIVE.** None — do the division on the slide or the whiteboard. This is the deliberately
wifi-proof centre of the talk: if every terminal in the room dies, this beat and the close
still carry it.

**FALLBACK.** Not applicable by design. Under no circumstances re-run the benchmark live.

> **Deleted from the brief and why (say it in Q&A if asked):** the `40,000 / 11,518 = 3.47
> effective cores, 87% efficiency` derivation is gone. Its numerator is CPU burned on two
> m5.large helpers; its denominator is wall time from a different machine's native run; and its
> divisor — four initiator cores — is recorded nowhere on disk. It also needed helper cores and
> local cores to be the same speed, which the very next beat denied. The `perfect 8-core = 5,000
> ms → 2.3× ceiling` is gone with it: the retained telemetry says `maxBusyHelpersCores=4`, there
> was never an eighth core, and this graph does not scale linearly anyway — which is exactly what
> the 3.28× row proves.

---

### 18:30–20:30 — THE LADDER, re-axised. Who authors the evidence?

**SCREEN.** Four rungs, forger on the right:
```
0  ASSERT A VALUE        cache_cleared_before_each_cold_sample: true
                         → anyone who writes the struct                      (one keystroke)
1  CHECK THE VALUE       the validator reads that boolean back
                         → the same person, the same keystroke — it is his literal
2  CHECK THE SHAPE       64 hex chars, non-empty path, file never opened
                         → anyone willing to type 64 zeros                    (a rewrite)
3  DERIVE FROM A DOCUMENT swf-cli opens the transcript, recomputes sha256, parses the header
                         → anyone who OWNS THE RECORDER.   ← 我们就停在这里 WE STOPPED HERE
                            (that is forgery three: 10 real files, 10 real digests, exit 0)
4  EVIDENCE YOU DID NOT AUTHOR   a signature, a countersignature, an instrument someone else runs
                         → 本仓库里没有任何东西在第 4 级。NOTHING IN THIS REPO IS ON RUNG 4.
                            except rustc, and rustc only guards the source.
```
Below, red: `留存文档 ≠ 可信文档。问题不是有没有文档，而是文档是谁写的。`
`A retained document is not an independent document. The question is not whether evidence exists.
It is who authored it.`

**SAY.**
> Here is the shape I promised, and here is where every fix we made today actually sits. And I
> have re-drawn it since I wrote the abstract, because the abstract was wrong.
>
> Rung zero: assert a value. The receipt says cache cleared, true. One keystroke.
>
> Rung one: check the value. Add a validator that reads the boolean. This is what everybody does,
> and it bought exactly nothing, because it was reading a literal we had written ourselves. Same
> person. Same keystroke.
>
> Rung two: check the shape. Sixty-four hex characters. Now he needs sixty-four zeros instead of
> the word true. We moved the hole about an inch.
>
> Rung three: stop letting the value be asserted at all. Delete the field. The only way such a
> value comes into existence is that `swf-cli` opens a transcript, recomputes its digest and
> parses its header. And I came here to tell you that this is where the ladder ends.
>
> It is not. It is where *we* stopped. Because rung three only costs the forger a document — and
> if he owns the recorder, my repository hands him ten. That is forgery three. Real files. Real
> digests. My own tool calling them corroborated. Exit zero.
>
> So the axis is not representability. It is authorship. **A check wins against exactly one
> adversary: the one who does not control its input.** When the thing being examined is written
> by the party you are defending against, it does not matter how many digests you recompute —
> you have built a mirror with a hash function in it.
>
> And rung four — evidence authored by somebody the forger does not control — appears **nowhere**
> in my repository. Nothing is signed. No transcript is bound to a machine, a user, or a clock
> that anyone else attests. I am not going to pretend a type system fixes that, because it
> doesn't. Rung four is a cryptographic and social problem, and I have not solved it.
>
> One more, because it is the same move somewhere completely different, and it is the fix I am
> proudest of. Incredibuild's build cache key includes rustc's output path. Our benchmark
> `mktemp`'d a fresh target directory for every sample — which destroyed cache reuse by
> construction, invisibly, and every one of our fifteen samples reported the same total of
> fifty-two lookups and one hit. The fix was not a check for did-we-get-enough-hits. The function
> that made the temp directory is *gone*. The configuration that could produce the wrong number
> no longer exists. That is a structural fix, and notice its adversary: it is not a forger. It is
> me, next month, having forgotten.

**LIVE.** The best live moment in the talk, and it is a grep:
```
grep -rn "cache_cleared_before_each" rust/crates/
```
Exactly **two** hits, both inside the guard test, both assembled with
`format!("cache_cleared_before_each_{}", "cold_sample")` so the guard does not match itself. The
field is gone from the program; the only mentions left are the test that forbids it. Then:
```
grep -n "fresh_target" scripts/robot-demo/ib-benchmark.sh
```
which returns only the comment recording that the function was removed. **The absence is the
demonstration.**

**FALLBACK.** Do **not** grep for `mktemp` — it is still in that file at line 130, for an
unrelated work directory, and the command would fail on stage. Do **not** cite the guard test by
line number; let grep print it. If the terminal is dead, the two-hit grep output goes on the
slide as a captured block.

> **Cut and why:** the `52 total / 52 hits (100% reuse)` figure. Fifteen real `cache.txt` files
> in the evidence tree say `total=52, hits=1`; `52/52` appears in exactly one place —
> `outputs/FACTORY-STORY.md:73`, a narrative document — with no artifact behind it. By this
> talk's own rule that is a rung-zero assertion, and it cannot be on a slide in *this* talk. It
> moves to §7 as `[PENDING]`.

---

### 20:30–23:00 — THE RUST BEAT. Breaking my own analogy, then line 147.

**SCREEN.** Left: `别检查非法状态，让它无法被表示。 Don't check for the illegal state. Make it
unrepresentable.` — `Option` instead of null · exhaustive `match` · private fields, so there is
no second constructor · `NonZeroU32` · `&mut` exclusivity.

Right, red, headed `还没做完 / NOT DONE — the diffs we have not written`:
```rust
// 1. the receipt side
fn parse(path: &Path, expected: Sha256) -> Result<CacheClear>   // the ONLY public constructor
                                                                // module-private field

// 2. the gate side
struct FreshnessVerdict(/* private */);
impl FreshnessVerdict { fn evaluate(age_ms: u64, policy: &Policy) -> Self { … } }  // ← the inner one
fn permit(FreshnessVerdict, StopVerdict, TimestampVerdict) -> Decision
```
Below, red: `a receipt is JSON — nobody has to use my struct` · `an adversarial pass still forged
a passing receipt AFTER transcript verification landed` · `rung 4 is not a type-system problem`

**SAY.**
> You have been watching a Rust idea for six minutes without me naming it. Don't check for the
> illegal state. Make it unrepresentable. `Option` instead of null, so there is no null to
> forget. Exhaustive `match`, so the missing case is a compile error and not a Tuesday. Private
> fields, so there is no second way to construct the thing.
>
> Now let me break my own analogy, in two places, before one of you does it for me.
>
> First: a receipt is JSON. Nobody is obliged to use my struct, and the compiler enforced
> nothing whatsoever about the attacker. What actually holds is smaller and more interesting —
> there is exactly one code path that can produce a cache-state value, and that path opens a
> file. Rust did not enforce that. Rust made it the *ergonomic* option: module privacy means the
> struct literal is impossible from outside, so the smart constructor is not merely available,
> it is the only door. In C you can write the smart constructor too. You just cannot stop anyone
> writing the struct literal next to it. That is a smaller claim than "Rust makes it
> unrepresentable," and it is the true one.
>
> Second, and worse for me: the regress does not terminate, and a type does not terminate it. I
> would love to tell you otherwise. My own repository refutes it in the same twenty-four hours.
>
> So what is the type system actually *for*, if it does not end the regress?
>
> Here is the answer I believe, and it is the whole talk. **Every check has an adversary.** My
> validator's adversary was *me*, and I own its input, so it never had a chance. But the type
> system's adversary is different. Its adversary is the next person to touch this file — a
> contributor, six months from now, at five in the afternoon, who controls the diff and nothing
> else. And that person **cannot author rustc.** The evidence is produced by an instrument
> outside his reach. That is rung four, and it is the only rung four I have.
>
> That is why a type wins where my digest lost. Not because it is stronger. Because its
> adversary is weaker — and its adversary is the one you actually face on almost every line you
> write.
>
> And now back to line one hundred and forty-seven.
>
> rustc could never have checked that two hundred and fifty was the right threshold. The age is a
> runtime quantity; no type system in practical use will help you there, and anyone who tells you
> otherwise is selling something. But rustc could have caught **the forgetting**. If `decide`
> returned a `Decision` whose permit constructor required a freshness verdict alongside the stop
> verdict and the timestamp verdict, then `Decision::Permit`, bare, would not compile. Not failed
> a test. Not failed a review. **Not compiled.**
>
> And I have to say the second half of that, because it is rung three all over again, one level
> down: making the *verdict* an argument only forces me to *name* one. A lazy implementer writes
> `FreshnessVerdict::Fresh` and discards the age and it compiles fine. The real fix needs the
> inner constructor too — `FreshnessVerdict` with a private field whose only way in is
> `evaluate(age_ms, policy)`. That is on the slide, in red, in the not-done column, because I
> only worked it out while an adversarial reader was taking this talk apart.
>
> Same file. Same demo. No new machinery. It is the first thing I am doing on the flight home.

**LIVE.**
```
grep -n "cache_state_is_never_asserted_by_construction" rust/crates/swf-cli/src/main.rs
```
then open the surrounding thirty lines, so nobody has to take the self-criticism on trust. It is
a test that reads its own source with `include_str!` and fails if the shape comes back — and it
locates the dispatch arm by an exact source string and `expect`s on it, so a restructure makes it
**panic** rather than quietly pass. Brittle in the one direction where brittleness is a virtue:
loudly. **No line number on the slide — let grep print it.**

**FALLBACK.** Static screenshot of the same lines, captured the morning of the talk. This beat has
no runtime dependency at all: it is a reading, and the reading is the point.

**Say this in this beat, unprompted, because someone will say it in Q&A otherwise:** the two
components that actually make this demo honest are `check-patch.py`, forty-seven lines of Python,
and `ib-benchmark.sh`, eight hundred and twenty-eight lines of bash. Rust owns the receipt parser.
It does not own the discipline. Say it before they do.

---

### 23:00–25:00 — CLOSE. Walk the ladder back down, then invite them to break it.

**SCREEN.** Five questions, stacked, bilingual, each answer struck through:
```
机器人安全吗？ How do you know the robot is safe?       the task succeeded.        the contract failed.
门写对了吗？   How do you know the gate is right?       contract tests.            the gate omitted rule 3.
基准是真的吗？ How do you know the benchmark is real?   the proof gate said PASS.  it certified a fabrication.
证明门有效吗？ How do you know the proof gate works?    we forged three classes.   the third still passes.
矩阵覆盖全吗？ How do you know the matrix covers it?    17 PASS.                   age_ms > 500 passes all 17.
IB 真的更慢？  How do you know Incredibuild is slower?  we measured it.            with --force-remote. wrongly.
```
Beneath, alone:
`可伪造的字段 = 证据由同一方书写的字段`
`the forgeable set = the fields whose evidence the same party authored`
Last slide: repo QR · `github.com/zozo123/rust-china-conf` · `3 [PENDING] · 1 forgery class open ·
evidence/ 中的运行目录未提交 run dirs are NOT committed — see README`

**SAY.**
> Let me walk it back down.
>
> How do you know the robot is safe? The task succeeded. No — the contract failed. How do you
> know the gate is right? Contract tests. No — the gate omitted rule three, in one line I read
> out loud to you. How do you know the benchmark is real? The proof gate said pass. No — it
> certified a fabrication. How do you know the proof gate works? We forged it three ways, and
> the third still passes tonight. How do you know the matrix covers the contract? Seventeen
> green. No — reject at five hundred and all seventeen still pass. How do you know Incredibuild
> is slower? We measured it. With force-remote. Wrongly.
>
> Six rungs. Five collapsed. And the temptation right here is to say *verify everything* — which
> is not an answer, it is just the ladder again with more rungs on it.
>
> [the closing paragraph, verbatim — §1]

**LIVE.** None. Stop talking. Do not add a thank-you slide after the last line. Repository URL
stays on screen through Q&A.

**FALLBACK.** Not applicable. If the deck is dead, deliver the final paragraph from memory. It is
the one part of this talk that must survive total equipment failure.

---

## §3. SLIDE LIST — one line per slide, one idea each

| # | slide | the single idea |
|---|---|---|
| 1 | the `stale_600ms` JSON row, full bleed | success = true, and it is the evidence itself, not a render of it |
| 2 | FAIL verdict ∥ `lib.rs:147` | the same run, read by the contract, is red — and the defect is literal |
| 3 | 250 ms ∥ 942 ms ∥ `= 3.8` | two clocks that will never meet; the threshold is mine, and it is too generous |
| 4 | check-patch / 17 PASS / digest chain, with the timing strip | machinery, and it feels good, and that feeling is the setup |
| 5 | one green `BUILD PROOF PASS` block on black | the benchmark's proof gate says yes |
| 6 | forgery table, three rows | 3 of 8 conditions were decoration; the fix moved the hole; the third is open |
| 7 | the no-op script + the ten real digests | real documents, real digests, exit 0, `ratio=119.949x` |
| 8 | AL6555 table + Build History counters + `-f` reveal | we measured four remote cores replacing every local one |
| 9 | `freshness_ms: [0, 50, 600]` + `age_ms > 500` | forgery four: "complete coverage matrix" checks three sampled values |
| 10 | j1/j10 table, two rows | cold saturates at 3.3×; warm is 1.08× — there is no parallelism to sell |
| 11 | the four-rung ladder, forger on the right | the axis is authorship, not representation; rung 4 is empty |
| 12 | `local_only` + `ib_cache enabled` one-word diff | distribution and caching are two knobs, and the vendor's schema says so |
| 13 | unrepresentable ∥ NOT DONE (both constructors, in red) | Rust makes rung 3 the cheap rung to build — and does not reach rung 4 |
| 14 | the six questions, struck through | five collapsed; the regress changes hands |
| 15 | QR + repo + "3 PENDING · 1 forgery open · run dirs not committed" | come break it |

**Slide 12 note.** It is a quotation, not a measurement — `/opt/incredibuild/data/ib_profile.xsd`
lives on the Linux host and does not exist on the laptop, so pre-render the excerpt. It belongs
at 18:30 or in Q&A as the "what to do instead" answer, not as its own beat; the h-model
projection that used to sit beside it is cut (see §6).

---

## §4. SPEAKER NOTES FOR THE THREE HARDEST MOMENTS

### 4.1 Admitting the forgeries (8:30–12:00)

The failure mode here is **performed humility**, and this room will smell it instantly. Four
rules:

1. **No apologetic framing.** Never "unfortunately," "I'm embarrassed to say," "full
   disclosure." Say the thing flat, in the past tense, with the number. "That receipt is a
   fabrication. I wrote it. It took about a minute." Then stop.
2. **Do not smile on the punchline.** The room will laugh at `/tmp/does-not-exist.txt` and at
   the three-line no-op script. Let them. Do not laugh with them — you are the one being
   laughed at, and accepting that silently is what buys the next fifteen minutes.
3. **Forgery three is not a cliffhanger, it is a concession.** Deliver it slower than the other
   two, and do not follow it with a recovery sentence. The beat ends on "so — how do you know
   the proof gate works? I don't." Then move.
4. **Own the smallest one hardest.** The `argv=$*`-after-`shift` bug is the most humiliating and
   the most relatable; it is the only place in the talk where you can be genuinely funny without
   undercutting yourself.

One line to have ready if the room goes cold: *"I'm told this is a strange thing to fly
fourteen hours to say. I think it's the only thing worth flying fourteen hours to say."*

### 4.2 Presenting a negative distribution result without sounding defensive (12:00–14:30, 16:00–18:30)

You work at Incredibuild. The room knows or will find out. The defensive versions all fail.

**Do:**
- Say "I work at Incredibuild. That number stays on the slide." Once, early, flat, then never
  mention it again. It converts the conflict of interest from a liability into the strongest
  credibility you have.
- Make the finding about **your workload**, not their product. `3.28× on ten cores` and
  `1.08× warm` are facts about a dependency graph of forty-five crates. Say "this is arithmetic
  about my dependency graph" — that is true, it is generous, and it is *more* damning than an
  accusation because it is checkable.
- Volunteer the `-f` mistake **before** you state the ratio's meaning. The order matters: the
  correction has to arrive while the audience still trusts you, not after they've caught you.
- Keep "the result is still negative and I am not walking it back." The temptation under
  pressure is to soften it into "inconclusive." Don't — you measured it five times per mode in
  rotating order, and hedging now would be its own small forgery.

**Don't:**
- Don't say "in fairness to Incredibuild." Every such clause reads as a company man protecting
  his employer.
- Don't claim you predicted the loss in advance. You didn't. The version of that claim that is
  true is narrow and worth saying: *"the arithmetic was available before the benchmark ran, and
  we did not do it."*
- Don't offer the cache pivot as a rescue. Offer it as the **next measurement**, marked pending,
  with the command. A pivot presented as a result is exactly the move this talk exists to
  condemn.

### 4.3 The close (23:00–25:00)

- **Slow down by about 20%** for the six-question walk-down. It is the only recapitulation in
  the talk and the audience needs the beat-count to land — question, answer, strike-through.
- **The stumble to avoid** is racing into the final paragraph. Put a full two seconds of silence
  after "with force-remote, wrongly." Then "Six rungs. Five collapsed."
- **Do not add anything after the last sentence.** No thank-you, no "questions?", no slide.
  Silence, then the chair. If you need a physical cue: step back from the lectern on "one more
  day" and stop.
- **If you are over time**, the close is not what you cut. Cut 5:30 to ninety seconds and cut
  the microduck side panel entirely. The close is load-bearing.
- **The one sentence you must not fumble:** "Where that name is your own team, the check is a
  mirror." Rehearse it cold, twenty times, standing up.

---

## §5. Q&A SHEET — the eight hardest questions, with honest answers

**Q1. "Did you just ship a broken verifier?"**
Yes. Twice, and the third is broken right now. The first shipped with three of eight conditions
that could not fire. The second checked a digest's shape without opening the file. The third
opens the file, recomputes the digest, and still passes a receipt whose ten transcripts were
produced by the repository's own recorder wrapped around a script that does nothing. What is
*not* broken is the disclosure: `swf-cli` prints its own `NOT CHECKED` paragraph on every run,
naming the counters it cannot corroborate. That is the difference between a bug and a lie, and
it is the only defence I am offering.

**Q2. "Then why should we trust any number in this talk?"**
Don't — grade them. Three tiers, and I will tell you which tier each slide is.
*Tier one, you can check on the spot:* the two build speedups (3.28× and 1.08×), because they
are two numbers from one instrument and the conclusion is a division you just did.
*Tier two, you can check if you clone:* `lib.rs:147`, the three red contract tests, the
coverage matrix's `[0, 50, 600]`, `check-patch.py`'s allowlist, the two-hit grep. All are in the
public repository and reproduce in under a minute.
*Tier three, my word alone:* everything from the AL6555 grid — the 11,518 / 23,173 / 22,297
medians, the Build History counters. Those run directories are `.gitignore`d. They are rung-zero
assertions from where you sit, and I would not accept them from a speaker either.

**Q3. "You said the repository is public and the forged receipts are in it. Are they?"**
[If §7-A landed:] Yes, `demo/forged/`, with a README naming which validator each one defeats and
which test now refuses it. [If it did not:] No — and that is a real answer, not a dodge. `evidence/*/`
is gitignored at line 19; `git ls-files evidence/` returns two files. What is public is the code:
the gate, the seeded line, the contract tests, `check-patch.py`, both profiles, the scripts, and
`swf-cli` including its own `NOT CHECKED` disclosure. What is not public is every run directory
whose id appears in my footers. Committing them is the first item on my list and I will post the
commit.

**Q4. "Isn't 'make illegal states unrepresentable' just the standard Rust talk with a robot on it?"**
The standard version claims the type system ends the argument. This talk's evidence is that it
does not: my rung-three fix — the one that deleted the field and derived the value from a parsed
document — was defeated the same day. What I am adding is the adversary model. A check wins
against exactly the adversary who does not control its input. The type system's adversary is the
next contributor, who controls the diff and cannot author the compiler, so the type system wins
that fight completely and permanently. My validator's adversary was me, and I owned its input, so
no amount of structure could have saved it. That is a narrower claim than the standard talk makes
and I think it is the one that is true.

**Q5. "Your 3.28× was measured on a ten-core Mac. Doesn't that number change on other hardware?"**
The wall times do. The *shape* does not, and the shape is the claim. Ten cores bought 3.28× on the
cold build, which means the graph's critical path — not the core count — is the binding
constraint; more cores move that number very little, which is exactly why the AL6555 result came
out negative with four helper cores. And the warm row, 1.08×, is hardware-independent in its
conclusion: a one-file rebuild has almost no concurrency in it on any machine. If you want the
honest limitation: I have not measured this on a large workspace. microduck is 576 locked
packages and 355 compilation units — 6.8× my units, 8.5× my cold wall on the *same* Mac — and I
expect the shape to hold and the constant to change. I have not run it. It is on the pending list.

**Q6. "You work at Incredibuild and you're presenting a result where Incredibuild loses. Which is it — bad faith, or bad measurement?"**
Bad measurement, then a real negative. The run you saw passed `--force-remote`, which does not
add remote capacity, it *replaces* local capacity — their own Build History recorded
`maxInitiatorCores=0` for every sample, which is that flag, in their numbers. So the 2.01× figure
is four helper cores against four idle local ones, and it is not a fair characterisation of the
product. The reason I still call the result negative is the *other* measurement, the one with no
Incredibuild in it at all: this workload has 3.3× of parallelism cold and 1.08× warm. A
distributor sells parallelism. There is very little here to buy, whoever is selling it. And the
thing I should have measured — the Build Cache, with distribution off, which is one word in the
profile — I have not measured. That is the honest position.

**Q7. "You called a seventeen-scenario matrix 'complete coverage' and it doesn't test the boundary. How many more of these are there?"**
I don't know, and the fact that I found this one eighteen hours before standing here is the
answer to the question you are really asking. What I can tell you is the mechanism: the matrix
samples three freshness values and the threshold sits between two of them, so it is a check on
values, and a check on values only covers the values somebody thought to list. The contract tests
*do* pin 250 and 251 — they are two of the three red tests on my second slide — so the system
catches it even though the matrix does not. If you want the structural fix rather than another
value: the freshness axis should be derived from the policy constant, not typed into a JSON file
next to it. That diff is not written.

**Q8. "What would rung four actually look like, concretely?"**
For the receipt: a signature from a key the person writing the receipt does not hold — a CI
runner's attestation, a hardware key, a build service countersigning its own Build History
response with a digest bound into the receipt. Note that the Build History JSON *is* retained on
disk, in `build-proof/raw/*.history.json`; what is missing is not the document, it is the
binding. Nothing digests it into the receipt, so `build-proof` cannot re-read it. That is a schema
v3 and it is maybe a day of work. For the *source*, rung four already exists and you all use it
every day: it is rustc. Which is the small, unglamorous point I actually came to make — you
already own one instrument your colleagues cannot forge, and most teams use it for memory safety
and nothing else.

*Two more, held in reserve:*
**"Why not just use sccache / a remote cache?"** — Probably right, and untested. The measurement
that would settle it is the cache-only profile in §7-C, and it is one word different from the one
I ran.
**"Is the demo robot real?"** — It is robosuite 1.5.2 on MuJoCo 3.9.0, software-in-the-loop, and
the run records that string in all seventeen rows. There is also a kinematic mock in the repo and
I have been careful all talk to say which is which, because conflating them would be the same
category of error as everything else I have shown you.

---

## §6. WHAT MUST NEVER BE SAID ON STAGE — with the true replacement

| ✗ never say | ✓ say instead |
|---|---|
| `3.47 effective cores` / `87% parallel efficiency` | `3.28× on ten cores` — one machine, one tool, `summary-native.txt` |
| `perfect 8-core distribution = 5,000 ms → 2.3× ceiling` | the telemetry says `maxBusyHelpersCores=4`; and this graph does not scale linearly, which is what 3.28× proves |
| `the arithmetic predicted the loss before the benchmark ran` | `the arithmetic was available before the benchmark ran, and we did not do it` |
| `h = 0.98 → 230 ms, a rebuild inside the freshness budget` | `942 ms measured, = 3.8 freshness budgets. The rebuild never fits inside the interval it protects.` (the h-model at h=0 is `40,000/3.47`, which is 11,518 **by definition** — a model reading back its own literal) |
| `every forgeable field is a field with no retained source document` | `every forgeable field is a field whose evidence the same party authored` |
| `the Build History counters have no retained source document` | `they have one — `build-proof/raw/*.history.json` — but nothing **binds** it to the receipt, so build-proof cannot re-read it` |
| `52 total / 52 hits, 100% reuse` | `[PENDING]`. Fifteen real `cache.txt` files say `total=52, hits=1`; 52/52 has no artifact |
| `the gate worked. Once.` | it misfired on cargo's `rustc -vV` self-hit; the repo now ships `--empty-cache-hit-floor 1` for exactly that |
| `17 → 33 tests` | `8 → 17 → 42` (measured today: `test result: ok. 42 passed`) |
| `11,518 ms is how long this repository takes to build` (as a Mac/robot-half number) | AL6555 Linux grid, whose own preflight says `no sim venv — mock backend only`. The Mac's number for this run is 13,959 ms cold / 13,034 ms warm |
| `250 ms comes from microduck, not from me` | `lib.rs:23` calls it an illustrative demo value. It is mine, and against a 50 Hz loop it is 12.5 cycles **too generous** |
| `12.5 missed control cycles` (over a shot of the Panda arm) | `5 ticks here` (verify_run.py TICK_NS = 50 ms, 20 Hz); `12.5 cycles on a real 50 Hz biped` |
| `576 packages vs 45 = 12.8× the compilation units` | `355 vs ~52 compilation units = 6.8×`, and `59.23 s vs 6.98 s = 8.5×` cold wall **on the same Mac** |
| `microduck: 8.7k stars, Apache-2.0, 15 servos, btd/padd/mediad/tofd, cargo-zigbuild, RK3566, kinematics in 4.13 s` | only the four traceable facts: 23 workspace members, 576 locked packages, ~117k lines, robotd 50 Hz with a self-reported missed-tick count |
| `run local-e2e-20260924T125134` | `…-runner-a` or `…-runner-b`. The bare id is not a directory and `validate.sh` rejects it |
| `录制回放 RECORDED REPLAY · robosuite/MuJoCo SIL` over `docs/demo/index.html` | it is yesterday's run (`ec2-e2e-20260923-160725`) and its own copy calls the arm schematic. Label it `telemetry replay, schematic arm` or don't show it |
| playing `docs/assets/robot-lift.mp4` under a stale-episode caption | its README says **fresh-lift** frames. It is a different episode |
| `0.87 s` on the single-scenario validate line | that figure is runner-B's full validation inside the rehearse log; measured standalone today: 0.196 s and 0.128 s. Put no stopwatch on the slide |
| `zero infrastructure failures` stacked under the verifier's PASS line | that phrase is `swf-cli`'s matrix runner, not `verify_run.py`. Say `17 scenarios, 17 PASS` |
| `complete coverage matrix` as evidence of boundary coverage | the matrix samples `[0, 50, 600]`; the boundary lives in `tests/contract.rs` |
| pointing the audience at `docs/talk/` | it still contains all three banned items: `islo` (`slides.md:99`, `talk-25min.en.md:91,182`), `88/88` (`slides.en.md:165`), `21.426 / 21.948` (`slides.md:134–135`). **Scrub or don't point** |
| `the forged receipts are in the repo, including the one I cannot catch` | only after §7-A. Otherwise: `the code is public; the run directories are gitignored, and I'll post the commit that fixes that` |
| `build-proof proves this number is real` | it checks internal consistency. Forgeries pass. It says so itself, in its own `NOT CHECKED` paragraph |
| the mock backend described as robosuite SIL | the run records `robosuite 1.5.2 / mujoco 3.9.0` in all 17 rows; say the string |
| `islo` | nothing. It has zero working code and it is on the blacklist |
| `ib-benchmark.sh:176` or any `swf-cli` line number | let `grep` print it. The harness moved today; only `lib.rs:147` is promised, and it is verified on the working tree **and** origin |
| the live `receipt-zeros.json` demo | today's binary refuses it, by a named passing test. Use the no-op-transcript forgery |
| `jq … outputs/evidence-live/build-proof/samples.jsonl` to show inputs are committed | that file is 20 `"mode":"native"` rows from the Mac, and `outputs/` is outside the repo. It visibly proves the opposite. Cut the command |

---

## §7. [PENDING] REGISTER

Everything still unmeasured or unlanded, what produces it, and where it goes.

### Blocking before stage

**A. Commit the forged receipts.** Nothing named `*forg*` exists anywhere in the tree.
→ Produce: build `demo/forged/` containing (1) `receipt-literals.json` (forgery 1),
(2) `receipt-zeros.json` (forgery 2, with a README noting today's binary refuses it and naming
`the_receipt_that_printed_ratio_11_948x_from_invented_fields_is_now_refused`), (3)
`receipt-noop-transcripts.json` + its ten transcripts + the three-line no-op script (forgery 3,
**still passes**), and a `README.md` mapping each to the validator it defeats.
→ Lands on: slide 7, the 8:30 live command, and the closing line.
→ **If it does not land:** cut the live command (§2, 8:30 FALLBACK) and use the §6 replacement
for the closing sentence.

**B. Make the evidence public, or say it isn't.** `.gitignore:19` is `evidence/*/`;
`git ls-files evidence/` returns 2 files. Every run id in every footer is unverifiable from the
audience's seat.
→ Produce: `git add -f evidence/local-e2e-20260924T125134-runner-{a,b}
evidence/build-exp-20260924T100157` (plus the AL6555 grid dir, copied in from `outputs/evidence-live/`),
then push; **or** add a `README` line stating plainly which directories are not committed.
→ Lands on: every slide footer, slide 15, Q3.

**C. Scrub or unlink `docs/talk/`.** Contains all three blacklisted items.
→ Produce: fix the four files, or remove the `docs/talk/` pointer from slide 15.
→ Lands on: slide 15.

### Measurements

**D. Cache-only warm wall time.** Distribution off, Build Cache on — the measurement this talk
argues for and has never run.
→ Produce: `IB_FORCE_REMOTE=0 IB_ACCEL=cache-only scripts/robot-demo/ib-benchmark.sh` with
`rust/ib_profile.cache-only.xml` (`type="local_only"` + `<ib_cache enabled="true"/>`), a stable
`CARGO_TARGET_DIR` with wiped contents, 5 samples, and `build-proof` run with distribution
excluded (it refuses the receipt if `remote_tasks` or `remote_core_time` is anything but zero).
→ Lands on: slide 12, and Q6's last sentence.
→ **Comparator that already exists:** native warm one-file rebuild = **942 ms** median j10
(915–984). Any cache result must be read against that, not against the cold build.

**E. Hit fraction after a one-file change.** Whether a cache serves 51 of 52 units when one crate
changes.
→ Produce: same run as D, then `swf-cli robot-demo build-history` for the cache counters.
→ Lands on: slide 12.
→ **Caveat to say out loud:** `/ib/mnt/fscache` was recreated by a 09:56 reboot and currently
reports 28K used — the cache is empty and being re-established. A number taken before it warms
means nothing.

**F. The 52/52 cache-reuse figure.** Stated in `outputs/FACTORY-STORY.md:73` with no artifact;
fifteen real `cache.txt` files say `total=52, hits=1`.
→ Produce: a run against a stable target path whose `raw/*.cache.txt` shows the hits, retained.
→ Lands on: nowhere until it exists. Cut from slide 11 (§6).

**G. The AL6555 initiator's core count.** Not recorded anywhere; `method.txt` for that run has no
host line. Every "N effective cores" claim depended on it.
→ Produce: `nproc` on AL6555, written into `method.txt` by the harness.
→ Lands on: nowhere. The talk no longer needs it — that is why the 87% derivation was cut.

**H. microduck as the large-workload comparator.** 355 compilation units, 59.23 s cold on the
same 10-core Mac (6.8× the units, 8.5× the wall). Never built under Incredibuild, never cached.
→ Produce: cold + one-file-warm, native and cache-only, on the grid.
→ Lands on: the 4:00 side panel and Q5. **Until then say "I expect the shape to hold and the
constant to change. I have not run it."**

### Structural work named on stage as not done

**I.** `CacheClear` with a module-private field and `parse(path, expected_digest)` as the only
public constructor — slide 13, left column of NOT DONE.
**J.** `FreshnessVerdict` with a private field and `evaluate(age_ms, policy)` as its only way in,
*plus* a `Decision` permit constructor that requires all three verdicts — slide 13, right column.
Both are needed; naming only the outer one is rung three all over again.
**K.** Schema v3: `{path, sha256}` per counter document, so the retained `*.history.json` is
*bound* to the receipt and re-readable at proof time — Q8.
**L.** Derive the coverage matrix's freshness axis from `DEFAULT_MAX_OBSERVATION_AGE_MS` instead
of typing three values into JSON beside it — Q7.

---

## §8. NOTES FOR THE ZH SCRIPT (do not write it here — hand these to the translator)

The repo's EN and ZH speaker scripts are structurally 1:1 and must stay that way. Every change
below needs a matching ZH edit at the same beat index.

1. **New beat inserted at 14:30 (forgery four).** ZH needs a whole new section. Key terms to fix
   in advance: 覆盖矩阵 (coverage matrix), 边界 (boundary), 阈值 (threshold). Suggested heading:
   `第四次伪造：矩阵里的洞`.
2. **The 16:00 beat is entirely replaced.** Delete all 87% / 3.47 / 有效核心 / 并行效率 language.
   The new content is two speedup rows and one sentence: 分发卖的是并行度，而这个工作负载已经没有
   并行度可卖了。
3. **The ladder at 18:30 changes axis.** 可表示性 → 谁书写了证据 (authorship). Rung 3's label
   changes from "删除表示" to "由文档推导出来 —— 但记录器是谁的？". A new rung 4 row is added.
4. **The closing paragraph is rewritten** (§1 has the ZH verbatim). The old ZH ending asserting
   没有留存源文档 must be removed wherever it appears.
5. **The bridge at 4:00 is inverted.** The old ZH said 250 ms 来自真实机器人; the new version says
   250 ms 是我自己定的演示值，而且对 50 Hz 的机器人来说还宽松了 12.5 倍. This is a reversal of
   meaning, not a rewording — flag it for the translator explicitly.
6. **Delete from ZH slides the same blacklist items as EN** (`docs/talk/slides.md:99, 134–135,
   165`; `talk-25min.md:76, 79, 97, 103`).
7. **Subtitle strip**: the ZH strip carries the same三-number claim and the same "一台机器" error.
   Replace both with the corrected strip at the top of this document.
8. **Terminology to pin once and reuse:** 伪造 forgery · 收据 receipt · 记录 transcript ·
   摘要 digest · 时效 freshness · 派发 dispatch · 契约 contract · 构造函数 constructor ·
   模块私有 module-private.
