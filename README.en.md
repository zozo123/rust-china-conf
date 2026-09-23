# A Million Compiles. One Robot Hour.

[简体中文](README.md) · [English website](https://zozo123.github.io/rust-china-conf/en/) · [Recorded replay](https://zozo123.github.io/rust-china-conf/demo/index.html?play=seeded&lang=en)

**The robot succeeded. The freshness contract failed.**

A simulated Panda arm lifts a cube using observations that are 600 ms old.
The seeded Rust gate permits the motion even though its policy requires
observations no older than 250 ms. A repaired candidate rejects that stale
request and still completes the task with fresh observations.

This repository demonstrates the surrounding validation loop: retain the
failure, review a bounded patch, remove the first workspace, rebuild in a
fresh workspace, and execute independent checks against the resulting binary.

[Talk and speaker cues](docs/talk/talk-25min.en.md) ·
[Six-minute runbook](docs/talk/runbook-6min.en.md) ·
[Slide outline](docs/talk/slides.en.md) · [Architecture and readiness](docs/plan.md)

## Run the local checks

Requires the pinned Rust **1.92.0** toolchain and Python **3.10+**.
The mock backend needs no third-party Python packages.

```bash
python3 scripts/robot-demo/check.py
```

This checks the current checkout, including uncommitted source, in a disposable
copy. It requires exactly the three intentional seeded contract failures,
reproduces the stale-dispatch defect, confirms that verification rejects it,
then applies the reviewed patch only in the copy. The patched workspace tests,
negative protocol/verifier tests, and all 17 mock scenarios must pass.

Logs and exported artifacts remain under `evidence/check-<timestamp>/`.
Mock results demonstrate protocol and software behavior, **not MuJoCo physics**.

## Rehearse the runner lifecycle

Runner scripts use the **committed revision**. Commit your intended source
before rehearsing; use `check.py` while editing.

```bash
scripts/robot-demo/rehearse.sh my-rehearsal
```

A and B receive separate run IDs and fresh Cargo outputs. B consumes A's
exported base revision. The default candidate is `demo/fallback-patch.diff`;
the script **does not invoke an LLM**. To use an externally produced patch:

```bash
ROBOT_DEMO_PATCH_FILE=/absolute/path/candidate.patch \
  scripts/robot-demo/rehearse.sh agent-rehearsal
```

The allowlist permits changes only to the gate implementation. Protected tests,
scenario fixtures, simulator, and verifier remain outside the candidate patch.
Failed runs retain partial evidence and clean up their temporary worktrees.

Validate an existing complete matrix with:

```bash
scripts/robot-demo/validate.sh my-rehearsal-runner-b
# A deliberately selected single scenario:
scripts/robot-demo/validate.sh one-run --scenario fresh_lift
```

## Real simulator on the Linux IB initiator

```bash
uv venv --python 3.12 demo/robot-sim/.venv
uv pip install -p demo/robot-sim/.venv -r demo/robot-sim/requirements-linux.txt
REQUIRE_IB=1 ROBOT_DEMO_BACKEND=robosuite \
ROBOT_DEMO_PYTHON="$PWD/demo/robot-sim/.venv/bin/python3" \
  scripts/robot-demo/rehearse.sh linux-rehearsal
```

The archived Linux run used CPython 3.12.14, robosuite 1.5.2 and MuJoCo 3.9.0.
The dependency file targets Linux x86-64. Keep Rust and the Python simulator
together on that host; the presenter laptop displays their evidence.

### Controlled EC2 / Incredibuild proof

The historical `ib: true` records prove integration only. The new Rust-first
proof path runs the same patched candidate in three modes—native, IB with an
explicitly cleared user cache, and IB after seeding that cache from the parent
revision—with at least five independent samples per mode:

```bash
export IB_ALLOW_CLEAR_USER_CACHE=1
export IB_HISTORY_URL='https://<coordinator>:8000/api/builds?coordinatorId=<id>&version=1.5.0'
export IB_CLIENT_API_KEY='<local secret>'
export ROBOT_DEMO_PYTHON="$PWD/demo/robot-sim/.venv/bin/python3"
scripts/robot-demo/ec2-agentic-physical-ai.sh conference-proof
```

`swf-cli`, not a Python analysis script, parses Build History and cache
statistics, requires remote tasks and positive remote core time in every IB
sample, requires zero cold hits and positive parent-warmed hits, then reports
medians and ranges. Missing or ambiguous telemetry is a failure. The wrapper
also runs the complete robosuite behavior rehearsal and writes a timing receipt.
It deletes disposable workspaces, not EC2 instances.

## Stack and scope

| Component | Responsibility |
| --- | --- |
| Coding agent, when supplied externally | Propose a bounded implementation change |
| Rust gate | Simulated stop precedence, timestamp validity, segment-dispatch freshness |
| Rust application / CLI | Subprocess protocol, identity and ordering, timeouts, evidence, IB telemetry parsing and benchmark proof |
| Python + robosuite / MuJoCo | Narrow simulator adapter and scripted Panda Lift task using simulator-provided state |
| Incredibuild | Compilation through `ib_console`; Build History remote-task counters and cache statistics become proof inputs |
| Protected verifier | Complete scenario coverage, chronological trace checks, artifact identity |
| Runner scripts | Temporary Git worktrees, fresh outputs, export and cleanup |

The current implementation deletes **workspaces**, not Linux machines.
The verified deployment used EC2-backed hosts. **Islo is a planned provider**;
an islo credential does not enable remote dispatch in the checked-in scripts.

The gate checks before each motion segment; a segment contains multiple
simulator control steps. This does not establish continuous supervision,
physical emergency-stop behavior, hardware-in-the-loop, or robot safety.
The 250 ms threshold is an illustrative policy, not a hardware limit.

### Rust robotics ecosystem

[robotics.rs](https://robotics.rs/) catalogs Rust-native ROS, simulation,
planning, vision and device libraries. This demo deliberately keeps its
external surface smaller: its safety contract, lock-step host, evidence model,
IB telemetry parser and proof gate are Rust; Python remains only because the
selected robosuite task is Python-native.

[`nexus-robotics-os`](https://crates.io/crates/nexus-robotics-os) 4.2.0-rc.2
has a compatible capability/safety/evidence philosophy, but it is a release
candidate and explicitly does not claim HIL or physical validation. It is not
added merely for branding: doing so would duplicate this demo's runtime and
change the measured build graph. A future Nexus adapter should be a separate,
measured integration with its own evidence.

## What the historical evidence says

Run `ec2-e2e-20260923-160725` contains 17 simulator episodes:
10 lifts, 5 stale rejections, 1 simulated stop, and 1 timeout.
Its original revision reported 10/10 gate tests and 88/88 verifier checks.
Those counts do not describe the strengthened current suite.

Build phase A measured **21.426 s**, phase B **21.948 s**. B was **522 ms slower**.
Both metrics records indicate IB use. They do not establish helper execution,
controlled cache state, cache reuse, or a speedup. New metrics explicitly mark
cache reuse as unverified. The controlled benchmark remains outstanding.

The historical executable is not committed. Its digest identifies the recorded
artifact, but cloning the logs cannot independently recheck that binary.
A new rehearsal exports its executable for verification.
[Evidence notes](docs/examples/README.en.md)

## Website maintenance

Chinese and English landing pages are static, including their titles and share
metadata. Edit the shared template and translations under `scripts/site/`:

```bash
python3 scripts/site/build.py
python3 scripts/site/build.py --check
python3 -m http.server 8765 --directory docs
```

Open `docs/index.html` or `docs/en/index.html` directly for offline use.
The matrix is generated from the committed results. The robot video has native
play/pause controls and starts paused. The replay also supports manual stepping.

## Deliberate seeded defect

Do not remove the missing freshness check from `main`: its failure is the
conference fixture. Apply the fallback or candidate patch inside the validation
workflow. The current checkout remains a useful red-to-green demonstration.
