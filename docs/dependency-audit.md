# Demo dependencies: Rust, Incredibuild, and robot simulation

> Historical pre-implementation audit. For current integrated capabilities and validation scope, see [the readiness plan](plan.md) and [recorded evidence](examples/README.en.md).

Source snapshot: [`59339a3dc9d66403d95b3a2dd2129df90bbd3cc4`](https://github.com/zozo123/ariflow-swfactory/commit/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4). Fetched directly from the GitHub API and raw files on 2026-09-23. These are repository versions, not a claim that every package is the newest available release.

The user selected a simulator, so the proposed demo is software-in-the-loop (SIL). The selected simulation baseline is robosuite 1.5.2 with MuJoCo 3.9.0, alongside a proposed Rust supervisor. The factory inventory below is an audited existing repository snapshot; the robot bridge and supervisor are proposed additions, not existing integrated capabilities. Package resolution has been checked, but no simulator execution or IB acceleration benchmark was performed during this audit.

## Runtime and build constraints

- Workspace/application version: **2.2.0**; Rust edition **2021**; declared minimum Rust **1.82**. The `rust-toolchain.toml` channel is floating `stable` with `rustfmt` and `clippy`, so pinning an exact tested compiler version remains necessary. The declared minimum alone does not prove the current transitive graph compiles on that compiler.
- Python: **>=3.12,<3.13**.
- Cargo workspace: five internal crates. `Cargo.lock`: **261 package entries**, including workspace crates, transitive dependencies, and target-specific packages. This is not the count necessarily compiled by `cargo test -p swf-domain --locked`.
- `uv.lock`: **149 package entries**, spanning the recorded Python resolution including groups; this is not a requirement to install every group for the demo.
- Incredibuild is build infrastructure and is not a Cargo or Python dependency in these manifests.

Sources: [Cargo workspace](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/Cargo.toml), [toolchain](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/rust-toolchain.toml), [Python manifest](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/pyproject.toml).

## Rust direct dependency constraints and locked resolutions

Cargo manifest strings are compatible-version constraints, not exact pins. For example, `ring = "0.17.14"` is not the same as `ring = "=0.17.14"`. `Cargo.lock` records the resolution used by a locked build. A crate can also appear at an additional transitive version.

| Library | Declared constraint | Locked version(s) | Enabled feature overrides |
|---|---|---|---|
| `anyhow` | `1.0` | `1.0.104` | No workspace override |
| `assert_cmd` | `2` | `2.2.2` | No workspace override |
| `async-trait` | `0.1` | `0.1.92` | No workspace override |
| `chrono` | `0.4` | `0.4.45` | default features off, std, clock, serde |
| `clap` | `4` | `4.6.6` | derive, env, wrap_help |
| `clap_complete` | `4` | `4.6.9` | No workspace override |
| `crossterm` | `0.28` | `0.28.1` | No workspace override |
| `directories` | `6` | `6.0.0` | No workspace override |
| `futures-util` | `0.3` | `0.3.34` | No workspace override |
| `insta` | `1` | `1.48.0` | No workspace override |
| `predicates` | `3` | `3.1.4` | No workspace override |
| `ratatui` | `0.29` | `0.29.0` | No workspace override |
| `reqwest` | `0.12` | `0.12.28` | default features off, json, rustls-tls |
| `ring` | `0.17.14` | `0.17.14` | No workspace override |
| `serde` | `1` | `1.0.229` | derive |
| `serde_json` | `1` | `1.0.151` | No workspace override |
| `tempfile` | `3` | `3.27.0` | No workspace override |
| `thiserror` | `2.0` | `2.0.20` | No workspace override |
| `tokio` | `1` | `1.53.1` | rt-multi-thread, macros, sync, time, process, signal |
| `tokio-util` | `0.7` | `0.7.19` | No workspace override |
| `toml` | `0.9` | `0.9.12+spec-1.1.0` | No workspace override |
| `unicode-width` | `0.2` | `0.1.14`, `0.2.0` | No workspace override |
| `url` | `2` | `2.5.8` | No workspace override |
| `wiremock` | `0.6` | `0.6.5` | No workspace override |

For `unicode-width`, the workspace directly selects the `0.2` line; the `0.1.14` entry is an additional transitive resolution.

Sources: [Cargo.toml](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/Cargo.toml), [Cargo.lock](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/Cargo.lock).

## Direct dependencies by Rust crate

### swf-domain

Versioned contracts and the pure roll-ups the factory's operator interface renders.

- Runtime: `serde`, `serde_json`, `ring`, `chrono`, `toml`, `thiserror`.
- Development/test: `insta`.
- [Pinned manifest](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/rust/crates/swf-domain/Cargo.toml).

### swf-adapters

Service adapters: Airflow REST v2, GitHub through gh, islo, and artifact metrics.

- Runtime: `swf-domain`, `anyhow`, `async-trait`, `tokio-util`, `thiserror`, `serde`, `serde_json`, `chrono`, `reqwest`, `tokio`, `url`, `futures-util`.
- Development/test: `wiremock`, `tempfile`, `tokio`.
- Test-only feature additions for `tokio`: `rt-multi-thread`, `macros`, `sync`, `time`, `process`, `signal`, `test-util`.
- [Pinned manifest](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/rust/crates/swf-adapters/Cargo.toml).

### swf-app

The operations both interfaces expose: contexts, doctor, submit, gates, deliveries.

- Runtime: `swf-domain`, `swf-adapters`, `anyhow`, `async-trait`, `tokio-util`, `thiserror`, `serde`, `serde_json`, `chrono`, `toml`, `tokio`, `directories`, `tempfile`.
- Development/test: `tempfile`, `wiremock`, `tokio`.
- Test-only feature additions for `tokio`: `test-util`.
- [Pinned manifest](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/rust/crates/swf-app/Cargo.toml).

### swf-tui

The Ratatui factory: attention, jobs, evidence, approvals over the shared operations.

- Runtime: `swf-domain`, `swf-app`, `anyhow`, `chrono`, `ratatui`, `crossterm`, `tokio`, `unicode-width`, `serde_json`, `tokio-util`.
- Development/test: `insta`.
- [Pinned manifest](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/rust/crates/swf-tui/Cargo.toml).

### swf-cli

swf: one command that operates the whole factory, in scripts and at a terminal.

- Runtime: `swf-domain`, `swf-adapters`, `swf-app`, `swf-tui`, `anyhow`, `clap`, `clap_complete`, `serde`, `serde_json`, `chrono`, `tokio`, `tokio-util`.
- Development/test: `assert_cmd`, `predicates`, `tempfile`, `wiremock`, `tokio`.
- Test-only feature additions for `tokio`: `rt-multi-thread`, `macros`.
- [Pinned manifest](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/rust/crates/swf-cli/Cargo.toml).

The focused `swf-domain` validation directly uses `serde`, `serde_json`, `ring`, `chrono`, `toml`, and `thiserror`, plus `insta` for tests. It does not automatically compile every CLI/TUI dependency merely because those crates share the workspace.

## Python direct dependencies and groups

These are dependency groups (`[dependency-groups]`), not published `[project.optional-dependencies]` extras. Install only groups needed by the chosen orchestration path.

| Group | Library | Declared constraint | Locked version |
|---|---|---|---|
| Core | `pydantic` | `>=2.11` | `2.13.5` |
| Core | `pydantic-settings` | `>=2.10` | `2.15.0` |
| Core | `typer` | `>=0.16` | `0.27.2` |
| Core | `pyyaml` | `>=6` | `6.0.3` |
| Core | `textual` | `>=0.86` | `8.2.8` |
| dev | `pytest` | `>=8` | `9.1.1` |
| dev | `ruff` | `>=0.12` | `0.16.5` |
| dev | `pyte` | `>=0.8` | `0.8.2` |
| airflow | `apache-airflow` | `==3.3.2` | `3.3.2` |
| airflow | `apache-airflow-providers-standard` | `==1.19.0` | `1.19.0` |
| airflow | `apache-airflow-providers-common-ai` | `>=0.8.0` | `0.8.0` |
| astronomer-blueprint | `airflow-blueprint` | `==0.4.0` | `0.4.0` |

Build backend: `hatchling`, declared without a version constraint under `[build-system]`; no `hatchling` package entry appears in this `uv.lock`.

Sources: [pyproject.toml](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/pyproject.toml), [uv.lock](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/uv.lock).

## Proposed robot simulation layer

Use a simulated Panda arm and a robosuite manipulation task. Final task configuration, robot model assets, scripted motion sequence, and episode assertions still need implementation and rehearsal. The Rust supervisor must actually authorize or reject the next action before the Python bridge dispatches it to the environment; a parallel visualization is insufficient.

- Runtime target for dependency resolution: CPython 3.12, Linux x86_64.
- Selected candidate versions: robosuite **1.5.2**, MuJoCo **3.9.0**.
- Resolution succeeded for **29 Python packages**, recorded in `simulator-requirements-linux.txt` beside this file. This is an exact-version requirements snapshot, not a hash-locked or runtime-validated environment; build-backend and operating-system dependencies are not included.
- Use a separate simulator virtual environment from the factory orchestration environment. Their package sets have not been jointly resolved.
- robosuite's current source declares `mujoco>=3.3.0,<3.10` because MuJoCo 3.10 changed an API used by its controllers. The published robosuite 1.5.2 metadata only declares `mujoco>=3.3.0`, so the explicit upper-compatible MuJoCo selection is intentional. Selecting 3.9.0 avoids that documented boundary; it does not establish that every other runtime combination works.
- The published robosuite 1.5.2 metadata also requires `mink==0.0.5`; this differs from current upstream source, which makes mink optional. The requirements snapshot follows the published package.

Sources: [robosuite release](https://pypi.org/project/robosuite/1.5.2/), [release dependency metadata](https://pypi.org/pypi/robosuite/1.5.2/json), [current source manifest and compatibility limit](https://github.com/ARISE-Initiative/robosuite/blob/master/setup.py), [MuJoCo 3.9.0](https://pypi.org/project/mujoco/3.9.0/), [MuJoCo Python/viewer documentation](https://mujoco.readthedocs.io/en/stable/python.html).

### Simulation libraries

| Library/group | Purpose |
|---|---|
| `robosuite` | Robot models, task environments, controllers and observations |
| `mujoco` | Physics engine, Python bindings and viewer |
| `numpy`, `scipy` | Numerical arrays and scientific computation |
| `numba`, `llvmlite` | JIT compilation used by the simulation framework |
| `mink`, `qpsolvers`, `quadprog` | Kinematics/optimization dependencies of the selected published release |
| `opencv-python`, `pillow` | Image handling |
| `glfw`, `pyopengl` | Display and rendering bindings |
| `pynput`, `evdev`, `python-xlib` | Input handling and Linux backends |
| `pytest`, `pluggy`, `iniconfig`, `pygments`, `packaging` | Test framework and supporting packages |
| `absl-py`, `etils`, `fsspec`, `typing-extensions`, `zipp`, `six`, `termcolor`, `tqdm` | Logging, filesystem, compatibility and console utilities |

### Proposed Rust bridge

Keep the policy function independent of the simulator. Use the standard library for policy logic and its tests, then reuse existing workspace crates such as `serde`/`serde_json` for the observation/action protocol, `tokio` for process lifecycle and timeouts, `thiserror` for typed failures, `clap` for a command interface, and `ring` for artifact digests where needed. These are proposed uses, not additional packages already committed to the repository.

A line-delimited JSON protocol over a child process's standard input/output is sufficient for an initial local bridge. Keep simulator diagnostics on standard error. Treat simulator observation timestamps and a controlled simulation clock explicitly; do not compare timestamps from unrelated clock domains. Log the decision and actual action dispatch in one episode trace.

### System and infrastructure dependencies

| Environment | Requirements |
|---|---|
| Rust build runner | Pinned Linux image, exact Rust/Cargo toolchain, Git, native C/assembler toolchain and linker for native crate dependencies, IB initiator |
| IB services | Compatible coordinator and shared Build Cache service; helpers for demonstrated distributed work; provisioned feature access and network connectivity |
| Simulation host | CPython 3.12 environment, the 29 resolved packages, pinned robot/task assets, working graphics backend |
| Desktop rendering | Working OpenGL/display stack; GLFW bindings alone do not provision the system graphics driver |
| Headless rendering | A tested EGL or OSMesa configuration, selected for the host rather than installed indiscriminately |
| Optional orchestration | Factory dependencies and selected Airflow group; chosen agent client and disposable-runner provider |

MuJoCo's Python package includes the engine; no separate engine download is required for the wheel-based installation. A training GPU, JAX, PyTorch, HIL-SERL, ROS, physical robot SDK, and physical camera SDK are not dependencies of the proposed scripted simulation demo. GPU-backed rendering is an optional deployment choice, not an RL-training requirement.

IB accelerates supported build tasks, not the Python simulation steps. Install/prefetch the simulator environment outside the timed Rust build experiment. No runtime compatibility or performance result is claimed by dependency resolution alone.

### Remaining integration work

The inspected factory manifests do not contain the selected simulator, robot bridge, or robot-facing Rust supervisor. They must be added and tested before this is an end-to-end demo. The existing human approval gates (HITL) are distinct from physical hardware-in-the-loop (HIL); this simulator-only demonstration should be labeled SIL.

Before freezing the demo, pin the model/task assets, scenario inputs, Rust toolchain, runner image, IB configuration, graphics backend and simulator requirements. Run the stale/fresh/stop scenarios and confirm the tested Rust executable actually controls the simulation action path.

## Complete Cargo.lock package inventory

All 261 entries are listed below. The same name can occur at multiple versions. Registry entries are the lockfile resolution; workspace entries have no external source. Refer to the [pinned Cargo.lock](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/Cargo.lock) for dependency edges and checksums.

| Package | Version | Source |
|---|---|---|
| `aho-corasick` | `1.1.5` | crates.io registry |
| `allocator-api2` | `0.2.21` | crates.io registry |
| `android_system_properties` | `0.1.6` | crates.io registry |
| `anstream` | `1.0.0` | crates.io registry |
| `anstyle` | `1.0.14` | crates.io registry |
| `anstyle-parse` | `1.0.0` | crates.io registry |
| `anstyle-query` | `1.1.5` | crates.io registry |
| `anstyle-wincon` | `3.0.11` | crates.io registry |
| `anyhow` | `1.0.104` | crates.io registry |
| `assert-json-diff` | `2.0.2` | crates.io registry |
| `assert_cmd` | `2.2.2` | crates.io registry |
| `async-trait` | `0.1.92` | crates.io registry |
| `atomic-waker` | `1.1.2` | crates.io registry |
| `autocfg` | `1.5.1` | crates.io registry |
| `base64` | `0.22.1` | crates.io registry |
| `bitflags` | `2.13.1` | crates.io registry |
| `bstr` | `1.13.1` | crates.io registry |
| `bumpalo` | `3.20.3` | crates.io registry |
| `bytes` | `1.12.1` | crates.io registry |
| `cassowary` | `0.3.0` | crates.io registry |
| `castaway` | `0.2.4` | crates.io registry |
| `cc` | `1.4.5` | crates.io registry |
| `cfg-if` | `1.0.4` | crates.io registry |
| `cfg_aliases` | `0.2.2` | crates.io registry |
| `chacha20` | `0.10.2` | crates.io registry |
| `chrono` | `0.4.45` | crates.io registry |
| `clap` | `4.6.6` | crates.io registry |
| `clap_builder` | `4.6.6` | crates.io registry |
| `clap_complete` | `4.6.9` | crates.io registry |
| `clap_derive` | `4.6.4` | crates.io registry |
| `clap_lex` | `1.1.0` | crates.io registry |
| `colorchoice` | `1.0.5` | crates.io registry |
| `compact_str` | `0.8.2` | crates.io registry |
| `console` | `0.16.4` | crates.io registry |
| `core-foundation-sys` | `0.8.7` | crates.io registry |
| `cpufeatures` | `0.3.1` | crates.io registry |
| `crossterm` | `0.28.1` | crates.io registry |
| `crossterm_winapi` | `0.9.1` | crates.io registry |
| `darling` | `0.24.1` | crates.io registry |
| `darling_core` | `0.24.1` | crates.io registry |
| `darling_macro` | `0.24.1` | crates.io registry |
| `deadpool` | `0.12.3` | crates.io registry |
| `deadpool-runtime` | `0.1.4` | crates.io registry |
| `difflib` | `0.4.0` | crates.io registry |
| `directories` | `6.0.0` | crates.io registry |
| `dirs-sys` | `0.5.0` | crates.io registry |
| `displaydoc` | `0.2.7` | crates.io registry |
| `either` | `1.18.0` | crates.io registry |
| `encode_unicode` | `1.0.0` | crates.io registry |
| `equivalent` | `1.0.2` | crates.io registry |
| `errno` | `0.3.14` | crates.io registry |
| `fastrand` | `2.5.0` | crates.io registry |
| `find-msvc-tools` | `0.1.12` | crates.io registry |
| `float-cmp` | `0.10.0` | crates.io registry |
| `fnv` | `1.0.7` | crates.io registry |
| `foldhash` | `0.1.5` | crates.io registry |
| `form_urlencoded` | `1.2.2` | crates.io registry |
| `futures` | `0.3.34` | crates.io registry |
| `futures-channel` | `0.3.34` | crates.io registry |
| `futures-core` | `0.3.34` | crates.io registry |
| `futures-executor` | `0.3.34` | crates.io registry |
| `futures-io` | `0.3.34` | crates.io registry |
| `futures-macro` | `0.3.34` | crates.io registry |
| `futures-sink` | `0.3.34` | crates.io registry |
| `futures-task` | `0.3.34` | crates.io registry |
| `futures-util` | `0.3.34` | crates.io registry |
| `getrandom` | `0.2.17` | crates.io registry |
| `getrandom` | `0.4.3` | crates.io registry |
| `h2` | `0.4.19` | crates.io registry |
| `hashbrown` | `0.15.5` | crates.io registry |
| `hashbrown` | `0.17.1` | crates.io registry |
| `heck` | `0.5.0` | crates.io registry |
| `hermit-abi` | `0.5.3` | crates.io registry |
| `http` | `1.5.0` | crates.io registry |
| `http-body` | `1.1.0` | crates.io registry |
| `http-body-util` | `0.1.5` | crates.io registry |
| `httparse` | `1.10.1` | crates.io registry |
| `httpdate` | `1.0.3` | crates.io registry |
| `hyper` | `1.11.1` | crates.io registry |
| `hyper-rustls` | `0.27.9` | crates.io registry |
| `hyper-util` | `0.1.20` | crates.io registry |
| `iana-time-zone` | `0.1.65` | crates.io registry |
| `iana-time-zone-haiku` | `0.1.2` | crates.io registry |
| `icu_collections` | `2.3.0` | crates.io registry |
| `icu_locale_core` | `2.3.0` | crates.io registry |
| `icu_normalizer` | `2.3.0` | crates.io registry |
| `icu_normalizer_data` | `2.3.0` | crates.io registry |
| `icu_properties` | `2.3.0` | crates.io registry |
| `icu_properties_data` | `2.3.0` | crates.io registry |
| `icu_provider` | `2.3.1` | crates.io registry |
| `ident_case` | `1.0.1` | crates.io registry |
| `idna` | `1.1.0` | crates.io registry |
| `idna_adapter` | `1.2.2` | crates.io registry |
| `indexmap` | `2.14.2` | crates.io registry |
| `indoc` | `2.0.7` | crates.io registry |
| `insta` | `1.48.0` | crates.io registry |
| `instability` | `0.3.13` | crates.io registry |
| `ipnet` | `2.12.2` | crates.io registry |
| `is_terminal_polyfill` | `1.70.2` | crates.io registry |
| `itertools` | `0.13.0` | crates.io registry |
| `itoa` | `1.0.18` | crates.io registry |
| `js-sys` | `0.3.105` | crates.io registry |
| `lazy_static` | `1.5.0` | crates.io registry |
| `libc` | `0.2.189` | crates.io registry |
| `libredox` | `0.1.23` | crates.io registry |
| `linux-raw-sys` | `0.12.1` | crates.io registry |
| `linux-raw-sys` | `0.4.15` | crates.io registry |
| `litemap` | `0.8.3` | crates.io registry |
| `lock_api` | `0.4.14` | crates.io registry |
| `log` | `0.4.34` | crates.io registry |
| `lru` | `0.12.5` | crates.io registry |
| `lru-slab` | `0.1.2` | crates.io registry |
| `memchr` | `2.8.3` | crates.io registry |
| `mio` | `1.2.3` | crates.io registry |
| `normalize-line-endings` | `0.3.0` | crates.io registry |
| `num-traits` | `0.2.19` | crates.io registry |
| `num_cpus` | `1.17.0` | crates.io registry |
| `once_cell` | `1.21.4` | crates.io registry |
| `once_cell_polyfill` | `1.70.2` | crates.io registry |
| `option-ext` | `0.2.0` | crates.io registry |
| `parking_lot` | `0.12.5` | crates.io registry |
| `parking_lot_core` | `0.9.12` | crates.io registry |
| `paste` | `1.0.15` | crates.io registry |
| `percent-encoding` | `2.3.2` | crates.io registry |
| `pin-project-lite` | `0.2.17` | crates.io registry |
| `potential_utf` | `0.1.6` | crates.io registry |
| `predicates` | `3.1.4` | crates.io registry |
| `predicates-core` | `1.0.10` | crates.io registry |
| `predicates-tree` | `1.0.13` | crates.io registry |
| `proc-macro2` | `1.0.107` | crates.io registry |
| `quinn` | `0.11.11` | crates.io registry |
| `quinn-proto` | `0.11.17` | crates.io registry |
| `quinn-udp` | `0.5.15` | crates.io registry |
| `quote` | `1.0.47` | crates.io registry |
| `r-efi` | `6.0.0` | crates.io registry |
| `rand` | `0.10.2` | crates.io registry |
| `rand_core` | `0.10.1` | crates.io registry |
| `rand_pcg` | `0.10.2` | crates.io registry |
| `ratatui` | `0.29.0` | crates.io registry |
| `redox_syscall` | `0.5.18` | crates.io registry |
| `redox_users` | `0.5.2` | crates.io registry |
| `regex` | `1.13.1` | crates.io registry |
| `regex-automata` | `0.4.18` | crates.io registry |
| `regex-syntax` | `0.8.11` | crates.io registry |
| `reqwest` | `0.12.28` | crates.io registry |
| `ring` | `0.17.14` | crates.io registry |
| `rustc-hash` | `2.1.3` | crates.io registry |
| `rustix` | `0.38.44` | crates.io registry |
| `rustix` | `1.1.4` | crates.io registry |
| `rustls` | `0.23.43` | crates.io registry |
| `rustls-pki-types` | `1.15.1` | crates.io registry |
| `rustls-webpki` | `0.103.15` | crates.io registry |
| `rustversion` | `1.0.23` | crates.io registry |
| `ryu` | `1.0.23` | crates.io registry |
| `scopeguard` | `1.2.0` | crates.io registry |
| `serde` | `1.0.229` | crates.io registry |
| `serde_core` | `1.0.229` | crates.io registry |
| `serde_derive` | `1.0.229` | crates.io registry |
| `serde_json` | `1.0.151` | crates.io registry |
| `serde_spanned` | `1.1.1` | crates.io registry |
| `serde_urlencoded` | `0.7.1` | crates.io registry |
| `shlex` | `2.0.1` | crates.io registry |
| `signal-hook` | `0.3.18` | crates.io registry |
| `signal-hook-mio` | `0.2.5` | crates.io registry |
| `signal-hook-registry` | `1.4.8` | crates.io registry |
| `similar` | `2.7.0` | crates.io registry |
| `slab` | `0.4.12` | crates.io registry |
| `smallvec` | `1.16.0` | crates.io registry |
| `socket2` | `0.6.5` | crates.io registry |
| `stable_deref_trait` | `1.2.1` | crates.io registry |
| `static_assertions` | `1.1.0` | crates.io registry |
| `strsim` | `0.11.1` | crates.io registry |
| `strum` | `0.26.3` | crates.io registry |
| `strum_macros` | `0.26.4` | crates.io registry |
| `subtle` | `2.6.1` | crates.io registry |
| `swf-adapters` | `2.2.0` | workspace/local |
| `swf-app` | `2.2.0` | workspace/local |
| `swf-cli` | `2.2.0` | workspace/local |
| `swf-domain` | `2.2.0` | workspace/local |
| `swf-tui` | `2.2.0` | workspace/local |
| `syn` | `2.0.119` | crates.io registry |
| `syn` | `3.0.5` | crates.io registry |
| `sync_wrapper` | `1.0.2` | crates.io registry |
| `synstructure` | `0.13.2` | crates.io registry |
| `tempfile` | `3.27.0` | crates.io registry |
| `terminal_size` | `0.4.4` | crates.io registry |
| `termtree` | `0.5.1` | crates.io registry |
| `thiserror` | `2.0.20` | crates.io registry |
| `thiserror-impl` | `2.0.20` | crates.io registry |
| `tinystr` | `0.8.4` | crates.io registry |
| `tinyvec` | `1.13.2` | crates.io registry |
| `tinyvec_macros` | `0.1.1` | crates.io registry |
| `tokio` | `1.53.1` | crates.io registry |
| `tokio-macros` | `2.7.2` | crates.io registry |
| `tokio-rustls` | `0.26.5` | crates.io registry |
| `tokio-util` | `0.7.19` | crates.io registry |
| `toml` | `0.9.12+spec-1.1.0` | crates.io registry |
| `toml_datetime` | `0.7.5+spec-1.1.0` | crates.io registry |
| `toml_parser` | `1.1.3+spec-1.1.0` | crates.io registry |
| `toml_writer` | `1.1.2+spec-1.1.0` | crates.io registry |
| `tower` | `0.5.3` | crates.io registry |
| `tower-http` | `0.6.11` | crates.io registry |
| `tower-layer` | `0.3.3` | crates.io registry |
| `tower-service` | `0.3.3` | crates.io registry |
| `tracing` | `0.1.44` | crates.io registry |
| `tracing-core` | `0.1.36` | crates.io registry |
| `try-lock` | `0.2.5` | crates.io registry |
| `unicode-ident` | `1.0.24` | crates.io registry |
| `unicode-segmentation` | `1.13.3` | crates.io registry |
| `unicode-truncate` | `1.1.0` | crates.io registry |
| `unicode-width` | `0.1.14` | crates.io registry |
| `unicode-width` | `0.2.0` | crates.io registry |
| `untrusted` | `0.9.0` | crates.io registry |
| `url` | `2.5.8` | crates.io registry |
| `utf8_iter` | `1.0.4` | crates.io registry |
| `utf8parse` | `0.2.2` | crates.io registry |
| `wait-timeout` | `0.2.1` | crates.io registry |
| `want` | `0.3.1` | crates.io registry |
| `wasi` | `0.11.1+wasi-snapshot-preview1` | crates.io registry |
| `wasm-bindgen` | `0.2.128` | crates.io registry |
| `wasm-bindgen-futures` | `0.4.78` | crates.io registry |
| `wasm-bindgen-macro` | `0.2.128` | crates.io registry |
| `wasm-bindgen-macro-support` | `0.2.128` | crates.io registry |
| `wasm-bindgen-shared` | `0.2.128` | crates.io registry |
| `web-sys` | `0.3.105` | crates.io registry |
| `web-time` | `1.1.0` | crates.io registry |
| `webpki-roots` | `1.0.9` | crates.io registry |
| `winapi` | `0.3.9` | crates.io registry |
| `winapi-i686-pc-windows-gnu` | `0.4.0` | crates.io registry |
| `winapi-x86_64-pc-windows-gnu` | `0.4.0` | crates.io registry |
| `windows-core` | `0.62.2` | crates.io registry |
| `windows-implement` | `0.60.2` | crates.io registry |
| `windows-interface` | `0.59.3` | crates.io registry |
| `windows-link` | `0.2.1` | crates.io registry |
| `windows-result` | `0.4.1` | crates.io registry |
| `windows-strings` | `0.5.1` | crates.io registry |
| `windows-sys` | `0.52.0` | crates.io registry |
| `windows-sys` | `0.59.0` | crates.io registry |
| `windows-sys` | `0.61.2` | crates.io registry |
| `windows-targets` | `0.52.6` | crates.io registry |
| `windows_aarch64_gnullvm` | `0.52.6` | crates.io registry |
| `windows_aarch64_msvc` | `0.52.6` | crates.io registry |
| `windows_i686_gnu` | `0.52.6` | crates.io registry |
| `windows_i686_gnullvm` | `0.52.6` | crates.io registry |
| `windows_i686_msvc` | `0.52.6` | crates.io registry |
| `windows_x86_64_gnu` | `0.52.6` | crates.io registry |
| `windows_x86_64_gnullvm` | `0.52.6` | crates.io registry |
| `windows_x86_64_msvc` | `0.52.6` | crates.io registry |
| `winnow` | `0.7.15` | crates.io registry |
| `winnow` | `1.0.4` | crates.io registry |
| `wiremock` | `0.6.5` | crates.io registry |
| `writeable` | `0.6.4` | crates.io registry |
| `yoke` | `0.8.3` | crates.io registry |
| `yoke-derive` | `0.8.2` | crates.io registry |
| `zerofrom` | `0.1.8` | crates.io registry |
| `zerofrom-derive` | `0.1.7` | crates.io registry |
| `zeroize` | `1.9.0` | crates.io registry |
| `zerotrie` | `0.2.5` | crates.io registry |
| `zerovec` | `0.11.8` | crates.io registry |
| `zerovec-derive` | `0.11.6` | crates.io registry |
| `zmij` | `1.0.23` | crates.io registry |

## Complete uv.lock package inventory

All 149 package entries are listed for completeness. These include group and transitive packages; the selected install environment determines which are installed. Refer to the [pinned uv.lock](https://github.com/zozo123/ariflow-swfactory/blob/59339a3dc9d66403d95b3a2dd2129df90bbd3cc4/uv.lock) for markers, group membership, dependency edges, artifact URLs, and hashes.

| Package | Version | Source |
|---|---|---|
| `a2wsgi` | `1.10.10` | registry: https://pypi.org/simple |
| `aiosmtplib` | `5.1.2` | registry: https://pypi.org/simple |
| `aiosqlite` | `0.22.1` | registry: https://pypi.org/simple |
| `airflow-blueprint` | `0.4.0` | registry: https://pypi.org/simple |
| `alembic` | `1.19.1` | registry: https://pypi.org/simple |
| `annotated-doc` | `0.0.5` | registry: https://pypi.org/simple |
| `annotated-types` | `0.8.0` | registry: https://pypi.org/simple |
| `anyio` | `4.15.0` | registry: https://pypi.org/simple |
| `apache-airflow` | `3.3.2` | registry: https://pypi.org/simple |
| `apache-airflow-core` | `3.3.2` | registry: https://pypi.org/simple |
| `apache-airflow-providers-common-ai` | `0.8.0` | registry: https://pypi.org/simple |
| `apache-airflow-providers-common-compat` | `1.18.0` | registry: https://pypi.org/simple |
| `apache-airflow-providers-common-io` | `1.8.0` | registry: https://pypi.org/simple |
| `apache-airflow-providers-common-sql` | `2.1.1` | registry: https://pypi.org/simple |
| `apache-airflow-providers-smtp` | `3.0.3` | registry: https://pypi.org/simple |
| `apache-airflow-providers-standard` | `1.19.0` | registry: https://pypi.org/simple |
| `apache-airflow-task-sdk` | `1.3.2` | registry: https://pypi.org/simple |
| `argcomplete` | `3.7.2` | registry: https://pypi.org/simple |
| `arrow` | `1.4.0` | registry: https://pypi.org/simple |
| `asgiref` | `3.12.1` | registry: https://pypi.org/simple |
| `attrs` | `26.1.0` | registry: https://pypi.org/simple |
| `babel` | `2.18.0` | registry: https://pypi.org/simple |
| `cachetools` | `7.1.8` | registry: https://pypi.org/simple |
| `cadwyn` | `7.0.0` | registry: https://pypi.org/simple |
| `certifi` | `2026.7.22` | registry: https://pypi.org/simple |
| `cffi` | `2.1.1` | registry: https://pypi.org/simple |
| `charset-normalizer` | `3.5.1` | registry: https://pypi.org/simple |
| `click` | `8.5.0` | registry: https://pypi.org/simple |
| `colorama` | `0.4.6` | registry: https://pypi.org/simple |
| `colorlog` | `6.12.0` | registry: https://pypi.org/simple |
| `cron-descriptor` | `2.1.0` | registry: https://pypi.org/simple |
| `croniter` | `6.2.4` | registry: https://pypi.org/simple |
| `cryptography` | `50.0.1` | registry: https://pypi.org/simple |
| `deprecated` | `1.3.1` | registry: https://pypi.org/simple |
| `dill` | `0.4.1` | registry: https://pypi.org/simple |
| `dnspython` | `2.8.0` | registry: https://pypi.org/simple |
| `email-validator` | `2.3.0` | registry: https://pypi.org/simple |
| `fastapi` | `0.136.3` | registry: https://pypi.org/simple |
| `fastapi-cli` | `0.0.32` | registry: https://pypi.org/simple |
| `fsspec` | `2026.7.0` | registry: https://pypi.org/simple |
| `genai-prices` | `0.1.6` | registry: https://pypi.org/simple |
| `googleapis-common-protos` | `1.75.2` | registry: https://pypi.org/simple |
| `greenback` | `1.3.0` | registry: https://pypi.org/simple |
| `greenlet` | `3.5.5` | registry: https://pypi.org/simple |
| `griffelib` | `2.2.0` | registry: https://pypi.org/simple |
| `grpcio` | `1.83.1` | registry: https://pypi.org/simple |
| `h11` | `0.16.0` | registry: https://pypi.org/simple |
| `httpcore` | `1.0.9` | registry: https://pypi.org/simple |
| `httpcore2` | `2.12.0` | registry: https://pypi.org/simple |
| `httptools` | `0.8.0` | registry: https://pypi.org/simple |
| `httpx` | `0.28.1` | registry: https://pypi.org/simple |
| `httpx2` | `2.12.0` | registry: https://pypi.org/simple |
| `httpx2-jsfetch` | `1.0` | registry: https://pypi.org/simple |
| `idna` | `3.19` | registry: https://pypi.org/simple |
| `importlib-metadata` | `9.0.1` | registry: https://pypi.org/simple |
| `iniconfig` | `2.3.0` | registry: https://pypi.org/simple |
| `isoduration` | `20.11.0` | registry: https://pypi.org/simple |
| `itsdangerous` | `2.2.0` | registry: https://pypi.org/simple |
| `jinja2` | `3.1.6` | registry: https://pypi.org/simple |
| `jsonschema` | `4.26.0` | registry: https://pypi.org/simple |
| `jsonschema-specifications` | `2025.9.1` | registry: https://pypi.org/simple |
| `lazy-object-proxy` | `1.12.0` | registry: https://pypi.org/simple |
| `libcst` | `1.9.0` | registry: https://pypi.org/simple |
| `linkify-it-py` | `2.2.0` | registry: https://pypi.org/simple |
| `lockfile` | `0.12.2` | registry: https://pypi.org/simple |
| `logfire-api` | `4.41.0` | registry: https://pypi.org/simple |
| `mako` | `1.4.1` | registry: https://pypi.org/simple |
| `markdown-it-py` | `4.2.0` | registry: https://pypi.org/simple |
| `markupsafe` | `3.0.3` | registry: https://pypi.org/simple |
| `mdit-py-plugins` | `0.6.1` | registry: https://pypi.org/simple |
| `mdurl` | `0.1.2` | registry: https://pypi.org/simple |
| `methodtools` | `0.4.7` | registry: https://pypi.org/simple |
| `more-itertools` | `11.1.0` | registry: https://pypi.org/simple |
| `msgspec` | `0.21.1` | registry: https://pypi.org/simple |
| `natsort` | `8.4.0` | registry: https://pypi.org/simple |
| `opentelemetry-api` | `1.44.0` | registry: https://pypi.org/simple |
| `opentelemetry-exporter-otlp` | `1.44.0` | registry: https://pypi.org/simple |
| `opentelemetry-exporter-otlp-proto-common` | `1.44.0` | registry: https://pypi.org/simple |
| `opentelemetry-exporter-otlp-proto-grpc` | `1.44.0` | registry: https://pypi.org/simple |
| `opentelemetry-exporter-otlp-proto-http` | `1.44.0` | registry: https://pypi.org/simple |
| `opentelemetry-proto` | `1.44.0` | registry: https://pypi.org/simple |
| `opentelemetry-sdk` | `1.44.0` | registry: https://pypi.org/simple |
| `opentelemetry-semantic-conventions` | `0.65b0` | registry: https://pypi.org/simple |
| `outcome` | `1.3.0.post0` | registry: https://pypi.org/simple |
| `packaging` | `26.3` | registry: https://pypi.org/simple |
| `pathlib-abc` | `0.5.2` | registry: https://pypi.org/simple |
| `pathspec` | `1.1.1` | registry: https://pypi.org/simple |
| `pendulum` | `3.2.0` | registry: https://pypi.org/simple |
| `platformdirs` | `4.11.7` | registry: https://pypi.org/simple |
| `pluggy` | `1.6.0` | registry: https://pypi.org/simple |
| `protobuf` | `7.36.1` | registry: https://pypi.org/simple |
| `psutil` | `7.2.2` | registry: https://pypi.org/simple |
| `pycparser` | `3.0` | registry: https://pypi.org/simple |
| `pydantic` | `2.13.5` | registry: https://pypi.org/simple |
| `pydantic-ai-slim` | `2.38.0` | registry: https://pypi.org/simple |
| `pydantic-core` | `2.46.5` | registry: https://pypi.org/simple |
| `pydantic-extra-types` | `2.11.1` | registry: https://pypi.org/simple |
| `pydantic-graph` | `2.38.0` | registry: https://pypi.org/simple |
| `pydantic-settings` | `2.15.0` | registry: https://pypi.org/simple |
| `pygments` | `2.21.0` | registry: https://pypi.org/simple |
| `pygtrie` | `2.6.1` | registry: https://pypi.org/simple |
| `pyjwt` | `2.13.0` | registry: https://pypi.org/simple |
| `pyte` | `0.8.2` | registry: https://pypi.org/simple |
| `pytest` | `9.1.1` | registry: https://pypi.org/simple |
| `python-daemon` | `3.1.2` | registry: https://pypi.org/simple |
| `python-dateutil` | `2.9.0.post0` | registry: https://pypi.org/simple |
| `python-dotenv` | `1.2.3` | registry: https://pypi.org/simple |
| `python-multipart` | `0.0.32` | registry: https://pypi.org/simple |
| `python-slugify` | `8.0.4` | registry: https://pypi.org/simple |
| `pyyaml` | `6.0.3` | registry: https://pypi.org/simple |
| `referencing` | `0.37.0` | registry: https://pypi.org/simple |
| `requests` | `2.34.2` | registry: https://pypi.org/simple |
| `rich` | `15.0.0` | registry: https://pypi.org/simple |
| `rich-argparse` | `1.8.0` | registry: https://pypi.org/simple |
| `rich-toolkit` | `0.20.3` | registry: https://pypi.org/simple |
| `rpds-py` | `2026.6.3` | registry: https://pypi.org/simple |
| `ruff` | `0.16.5` | registry: https://pypi.org/simple |
| `setproctitle` | `1.3.7` | registry: https://pypi.org/simple |
| `shellingham` | `1.5.4` | registry: https://pypi.org/simple |
| `six` | `1.17.0` | registry: https://pypi.org/simple |
| `sniffio` | `1.3.1` | registry: https://pypi.org/simple |
| `sqlalchemy` | `2.0.52` | registry: https://pypi.org/simple |
| `sqlparse` | `0.6.0` | registry: https://pypi.org/simple |
| `starlette` | `1.6.0` | registry: https://pypi.org/simple |
| `structlog` | `26.1.0` | registry: https://pypi.org/simple |
| `svcs` | `26.2.0` | registry: https://pypi.org/simple |
| `swfactory` | `2.2.0` | editable: . |
| `swfactory-contract-fixtures` | `0.0.0` | virtual: tests/fixtures/contract |
| `tabulate` | `0.10.0` | registry: https://pypi.org/simple |
| `tenacity` | `9.1.4` | registry: https://pypi.org/simple |
| `termcolor` | `3.3.0` | registry: https://pypi.org/simple |
| `text-unidecode` | `1.3` | registry: https://pypi.org/simple |
| `textual` | `8.2.8` | registry: https://pypi.org/simple |
| `truststore` | `0.10.4` | registry: https://pypi.org/simple |
| `typer` | `0.27.2` | registry: https://pypi.org/simple |
| `typing-extensions` | `4.16.0` | registry: https://pypi.org/simple |
| `typing-inspection` | `0.4.4` | registry: https://pypi.org/simple |
| `tzdata` | `2026.3` | registry: https://pypi.org/simple |
| `universal-pathlib` | `0.3.10` | registry: https://pypi.org/simple |
| `urllib3` | `2.7.0` | registry: https://pypi.org/simple |
| `uuid6` | `2025.0.1` | registry: https://pypi.org/simple |
| `uvicorn` | `0.52.4` | registry: https://pypi.org/simple |
| `uvloop` | `0.22.1` | registry: https://pypi.org/simple |
| `watchfiles` | `1.2.0` | registry: https://pypi.org/simple |
| `wcwidth` | `0.8.3` | registry: https://pypi.org/simple |
| `websockets` | `17.1` | registry: https://pypi.org/simple |
| `wirerope` | `1.0.0` | registry: https://pypi.org/simple |
| `wrapt` | `2.4.0` | registry: https://pypi.org/simple |
| `zipp` | `4.1.0` | registry: https://pypi.org/simple |
