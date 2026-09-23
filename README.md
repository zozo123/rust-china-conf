# rust-china-conf

**百万次编译，一小时机器人。** — 一次性运行器，热 Cargo 工厂。

> 烧掉运行器。留下证据。放过机器人。

[![真实 robosuite Panda 机械臂举起方块](docs/assets/robot-lift.gif)](https://zozo123.github.io/rust-china-conf/)

**[打开网站](https://zozo123.github.io/rust-china-conf/)** ·
**[回放证据](https://zozo123.github.io/rust-china-conf/demo/?play=seeded)** ·
**[阅读 25 分钟讲稿](docs/talk/talk-25min.md)** ·
[English](README.en.md)

真实 robosuite / MuJoCo 帧。每一段接近、下降、抓取、举起，都先经过 Rust 门授权。不是另做的动画。

站点默认简体中文，右上角可切 EN。字体自托管，会场可离线打开 `docs/`。GitHub Pages 不是国内 CDN；网络不通时用本地副本。

## 闭环

![从头到证明的验证闭环](docs/assets/validation-loop.gif)

一次性运行器上的冷分布式构建 → 三项契约测试失败 → 在 600 ms 陈旧观测上允许四次派发 → 有界智能体补丁 → **运行器销毁** → 新运行器热重建 → 10/10 测试 → 陈旧派发为零 → 88/88 受保护检查绑定到可执行文件摘要。

两个构建阶段都走了 Incredibuild（证据里 `ib: true`）。冷阶段 21426 ms，热阶段 21948 ms，**热阶段慢了 522 ms。** 这证明加速路径在运行器被销毁后仍然接入；这不是加速测量，本仓库任何地方都不声称加速。能支撑该声称的受控基准写在 [`docs/talk/talk-25min.md`](docs/talk/talk-25min.md)，尚未运行。

从钉死的环境重新生成媒体：

```bash
scripts/robot-demo/record-gif.sh        # 真实仿真画面
scripts/robot-demo/record-loop-gif.sh   # 闭环走查（中文画面）
```

端到端 **软件在环（SIL）** 演示：仿真 Panda 机械臂举起方块（robosuite `Lift`）。Rust 监督器里故意植入缺陷，允许按陈旧观测做拾取。编码智能体修复缺陷。全新隔离运行器重建候选，**实际得到的可执行文件** 在仿真里拒绝陈旧回合、完成新鲜回合。

完整计划：[`docs/plan.md`](docs/plan.md) · 网站：
[`docs/index.html`](docs/index.html) · 讲稿：
[`docs/talk/talk-25min.md`](docs/talk/talk-25min.md) · 舞台手册：
[`docs/talk/runbook-6min.md`](docs/talk/runbook-6min.md)

## 一分钟故事

1. `robot-safety-gate` 是纯 Rust 决策契约：仿真急停最先；非法/未来时间戳拒绝；任务动作仅在观测于派发时不超过 250 ms 时允许。
2. **植入缺陷**（会上标记的夹具）：新鲜度检查被省略。延迟 600 ms 的观测流得到允许的拾取。没有崩溃——*受保护断言*抓住它。
3. 智能体拿到有界工单，产出 49 行补丁（审阅过的回退：[`demo/fallback-patch.diff`](demo/fallback-patch.diff)，分支 `agent/fix-stale-observation`）。
4. 运行器 A（隔离沙箱）被 **销毁**。运行器 B——新的隔离沙箱——把补丁打在精确基线上，重建，重跑受保护检查。
5. 补丁后的可执行文件：陈旧回合 → `rejected_stale`，任务派发 **零**；新鲜回合 → `cube_lifted`。受保护校验器：88/88。

## 快速开始

```bash
# 1. Rust 工具链（1.78+）；mock 后端需要 Python 3.10+
cargo build --manifest-path rust/Cargo.toml --locked

# 2. 在植入修订上跑失败的验收用例（mock 后端）
./rust/target/debug/swf-cli robot-demo run --scenario stale_600ms --backend mock --run-id demo

# 3. 完整弧：运行器 A（冷，失败）→ 补丁 → 运行器 B（热）→ 结论
scripts/robot-demo/rehearse.sh my-rehearsal
```

### 真实仿真后端（SIL）

```bash
uv venv --python 3.12 demo/robot-sim/.venv
uv pip install -p demo/robot-sim/.venv -r demo/robot-sim/requirements-linux.txt
ROBOT_DEMO_BACKEND=robosuite \
ROBOT_DEMO_PYTHON=$PWD/demo/robot-sim/.venv/bin/python3 \
  scripts/robot-demo/rehearse.sh sil-run
```

已在 CPython 3.12 / robosuite 1.5.2 / MuJoCo 3.9.0 上核验。钉死的依赖面向 Linux x86-64；彩排也可在 macOS arm64 上跑。

## 植入缺陷（会议夹具）

`main` 故意带着回归：`tests/contract.rs` 在陈旧观测用例上 **按设计失败**——这次失败就是演示。审阅过的修复作为 `demo/fallback-patch.diff` 和分支 `agent/fix-stale-observation` 提供。不要在脚本化智能体流程之外“修好” main。250 ms 阈值是演示策略，**不是** 实体机器人已确立的安全阈值。

## 架构

```
智能体 → 一次性运行器 → 热 Cargo 工厂 [IB] → 保留证据
       → 仿真 → 未来 HIL / 机器人门（未做）
```

| 组件 | 负责 |
|---|---|
| `rust/crates/robot-safety-gate` | 纯决策契约；受控时钟输入；带类型的拒绝 |
| `rust/crates/swf-app` / `swf-cli` | 场景身份、桥接子进程协议、决策日志、超时 |
| `demo/robot-sim/bridge.py` | 仿真状态、提案生成、授权执行、步进 |
| `demo/robot-sim/scripted_controller.py` | 排练过的接近/抓取/举起提案（无学习） |
| `demo/robot-sim/acceptance/verify_run.py` | **受保护**校验器：轨迹断言、产物身份 |
| `scripts/robot-demo/*.sh` | 运行器 A/B 生命周期：隔离 git-worktree 沙箱、全新输出、导出、拆除 |
| `evidence/<run-id>/` | 清单、事件、结果、构建指标、智能体上下文、产物摘要 |

协议不变量：同一时刻只有一个未完成任务动作；批准只对精确的 action + tick 有效；畸形/超时桥接 → 显式回合失败，从不默认授权；拒绝不产生派发；保持步进有标签，物理仍可前进。

## 诚实范围

- 已做：Rust 契约检查、桥接检查、仿真机器人场景。
- **未**做：硬件 HIL、实体验证、训练视觉（方块位姿是仿真器状态；摄像头画面给观众）。
- `mock` 后端是 CI/开发用的运动学替身，每条消息都有标记；mock 结果不是仿真结果。
- Incredibuild **只加速编译**；测试与场景检查始终重跑。没有 IB 包装时，脚本会标明并报告带标签的原生基线，而不是声称加速。
- 运行器是带全新 `CARGO_TARGET_DIR` 的本地隔离 git worktree。远程沙箱提供商（islo）可通过被 gitignore 的 `.env.local`（`ISLO_SANDBOX_KEY`）配置——永不提交。

## 覆盖矩阵

5 个可达位置 × 3 种新鲜度场景（0 / 50 / 600 ms）+ 仿真急停 + 协议超时 = 17 回合，全部由受保护校验器对照实际构建产物核验。真实 robosuite 彩排的示例证据：[`docs/examples/`](docs/examples/README.md)。
