# Microduck local demo — working, 2026-09-24

A Rust biped robot, real daemons, MuJoCo body, running entirely on this Mac. No hardware.

## It works. Verified output

```
$ scripts/duck-sim ctl health
robot     healthy
  loop      50.0 of 50.0 Hz · 721 ticks · 3 missed · last 0 ms ago
  bus       ok
  imu       ready
  battery   7.40 V (50%)
  motors    32 °C max (left_hip_yaw) · 32 °C mean
central   not signed in: reachable on its own network only

software
  updaterd  0.15.0    robotd  0.15.0    configd  0.15.0

$ scripts/duck-sim drive
== walking forward for 8 s
== stopped — the intent expires on its own
```

## Why this matters for the talk

**`loop 50.0 of 50.0 Hz · 721 ticks · 3 missed`**

That one line does something our current demo cannot. `robotd` runs a **50 Hz control loop — a
20 ms period** — and it *reports its own missed ticks*. So:

- The safety contract's **250 ms** observation-freshness threshold is **12.5 control periods**.
  In the current repo that threshold is disclaimed as "an illustrative policy, not a hardware
  limit." Against a real 50 Hz loop it stops being illustrative and becomes arithmetic.
- "Stale observation" acquires a unit the audience can feel: *twelve and a half cycles during
  which the robot acted on something it already knew was old.*
- The loop publishes `missed` itself, so deadline misses are observable rather than asserted —
  which is the same standard this talk demands of everything else.

**And as a build workload** (measured today, 10-core Mac):

| | rust-china-conf | microduck |
|---|---|---|
| workspace members | 3 | 23 |
| locked packages | 45 | **576** |
| Rust LOC | ~4k | **117k** |
| cold full build | 11.5 s (4 cores) | **59.23 s (10 cores), 355 units** |

Our workspace compiles in 11.5 s at 87% parallel efficiency — there is almost nothing for a
cache or a distributor to win. Microduck is ~5× the build on a machine with 2.5× the cores, so on
the 4-core grid initiator expect roughly 2–3 minutes. **That is a workload where cache reuse has
room to show a number worth putting on a slide.**

## Exact repro

```bash
# 1. the two repos
git clone --depth 1 https://github.com/pollen-robotics/microduck.git
git clone --depth 1 https://github.com/pollen-robotics/microduck_rl.git

# 2. the simulator venv (MuJoCo 3.10.0 + onnxruntime 1.24.4)
cd microduck_rl && uv sync

# 3. THE GOTCHA — see below
ln -sf "$(dirname "$(readlink -f "$(uv python find 3.12)")")/../lib/libpython3.12.dylib" \
       .venv/libpython3.12.dylib

# 4. run it
cd ../microduck
export DUCK_SIM_RL=$PWD/../microduck_rl
export HEADLESS=1            # omit for a MuJoCo window
scripts/duck-sim             # daemons build from main, duck stands up
scripts/duck-sim ctl health
scripts/duck-sim drive
scripts/duck-sim down
```

## The gotcha, recorded because it cost three attempts

On macOS with a **uv-managed Python**, the body server dlopens `.venv/bin/python` and needs a
shared `libpython3.12.dylib`. uv's standalone CPython does not expose one along the baked-in
rpath, so it fails with:

```
Library not loaded: @rpath/libpython3.12.dylib
```

`DYLD_FALLBACK_LIBRARY_PATH` does **not** fix it — the lookup is an `@rpath` resolution and dyld
only tries the rpath list. But that list includes `.venv/bin/../libpython3.12.dylib`, so
symlinking the real dylib to `.venv/libpython3.12.dylib` resolves it. A system or Homebrew
Python 3.12 would avoid the problem entirely.

`mjpython` is required for the **windowed** viewer on macOS (MuJoCo's passive viewer must own the
main thread); `HEADLESS=1` avoids needing it.

## What I would and would not do with this before the talk

**Would:** use microduck as the *acceleration workload* on the grid — 576 packages instead of 45.
Low risk, and it is where a real cache speedup can appear.

**Would:** borrow the 50 Hz / 20 ms framing to make the 250 ms threshold physical.

**Would not:** port the safety gate, the bounded-patch allowlist, the protected verifier and the
evidence schema onto microduck before this talk. That is a substantial rewrite, and the current
robotics demo already passes 17/17 with a sha256-bound verifier. Trading a working demo for an
unproven one is the wrong bet this close to a stage.

**Status:** the duck is currently **up** on this machine. `scripts/duck-sim down` stops it.
