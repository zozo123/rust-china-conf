# 百万次编译，一小时机器人

[English](README.en.md) · [中文网站](https://zozo123.github.io/rust-china-conf/) · [记录回放](https://zozo123.github.io/rust-china-conf/demo/index.html?play=seeded&lang=zh)

**机器人成功了，观测时效契约却失败了。**

仿真 Panda 机械臂使用 600 ms 前的观测举起方块。植入缺陷的 Rust 决策门放行了运动，
而策略要求观测年龄不得超过 250 ms。修复后的候选版本拒绝陈旧请求，同时仍能在新鲜观测下完成任务。

本仓库演示围绕这个改动的验证流程：保留失败记录，审查受约束的补丁，删除第一个工作区，
在全新工作区重建，并对实际生成的可执行文件执行独立检查。

[讲稿与演讲提示](docs/talk/talk-25min.md) · [六分钟舞台手册](docs/talk/runbook-6min.md) ·
[幻灯片提纲](docs/talk/slides.md) · [架构与准备事项](docs/plan.md)

## 本地检查

需要固定的 Rust **1.92.0** 工具链和 Python **3.10+**。mock 后端无需第三方 Python 包。

```bash
python3 scripts/robot-demo/check.py
```

该命令在临时副本中检查当前工作区，包括尚未提交的源码。它确认恰好出现三个预期的植入缺陷测试失败，
复现陈旧观测仍被派发的问题，并确认校验器拒绝该证据。随后只在副本中应用已审阅的补丁。
修复后的工作区测试、协议与校验器反例测试，以及全部 17 个 mock 场景均须通过。

日志与导出的产物保留在 `evidence/check-<timestamp>/`。
mock 验证协议与软件行为，**不能代表 MuJoCo 物理仿真结果**。

## 彩排运行器流程

运行器脚本使用**已提交的修订**。彩排前请提交准备运行的源码；编辑期间使用 `check.py`。

```bash
scripts/robot-demo/rehearse.sh my-rehearsal
```

A 和 B 使用独立运行编号和全新 Cargo 输出。B 使用 A 导出的基线修订。
默认候选是 `demo/fallback-patch.diff`；脚本**不会调用大语言模型**。
外部智能体生成的补丁可以显式传入：

```bash
ROBOT_DEMO_PATCH_FILE=/absolute/path/candidate.patch \
  scripts/robot-demo/rehearse.sh agent-rehearsal
```

白名单只允许修改决策门实现。候选补丁不能修改受保护测试、场景、仿真器或校验器。
运行失败时保留部分证据，并清理临时工作区。

```bash
# 校验完整矩阵
scripts/robot-demo/validate.sh my-rehearsal-runner-b
# 显式选择单个场景
scripts/robot-demo/validate.sh one-run --scenario fresh_lift
```

## 在 Linux IB 发起机上运行真实仿真

```bash
uv venv --python 3.12 demo/robot-sim/.venv
uv pip install -p demo/robot-sim/.venv -r demo/robot-sim/requirements-linux.txt
REQUIRE_IB=1 ROBOT_DEMO_BACKEND=robosuite \
ROBOT_DEMO_PYTHON="$PWD/demo/robot-sim/.venv/bin/python3" \
  scripts/robot-demo/rehearse.sh linux-rehearsal
```

归档的 Linux 运行使用 CPython 3.12.14、robosuite 1.5.2 和 MuJoCo 3.9.0。
依赖文件面向 Linux x86-64。Rust 和 Python 仿真器应在同一主机运行，演讲电脑负责展示证据。

### 受控 EC2 / Incredibuild 证明

历史记录中的 `ib: true` 只能证明集成。新的 Rust 优先证明路径让同一个补丁候选按三种模式运行：
原生构建、显式清空用户缓存后的 IB 构建、以及先用父修订预热缓存再构建候选。
每种模式至少五个独立样本：

```bash
export IB_ALLOW_CLEAR_USER_CACHE=1
export IB_HISTORY_URL='https://<coordinator>:8000/api/builds?coordinatorId=<id>&version=1.5.0'
export IB_CLIENT_API_KEY='<本地密钥>'
export ROBOT_DEMO_PYTHON="$PWD/demo/robot-sim/.venv/bin/python3"
scripts/robot-demo/ec2-agentic-physical-ai.sh conference-proof
```

解析 Build History 和缓存统计的是 `swf-cli`，不是 Python 分析脚本。每个 IB 样本都必须有远程任务与正的远程核时；
冷样本必须零命中，父修订预热样本必须有命中；之后才报告中位数和范围。遥测缺失或歧义都会失败。
包装脚本还会运行完整 robosuite 行为彩排并写入分阶段计时收据。它删除一次性工作区，不会销毁 EC2 实例。

## 技术栈与范围

| 组件 | 职责 |
| --- | --- |
| 外部编码智能体 | 提出受约束的实现改动 |
| Rust 决策门 | 仿真停止优先、时间戳有效性、每段运动派发前的时效检查 |
| Rust 应用与 CLI | 子进程协议、身份与顺序检查、超时、证据、IB 遥测解析与基准证明 |
| Python + robosuite / MuJoCo | 狭窄的仿真适配器，使用仿真器状态执行脚本化 Panda Lift |
| Incredibuild | 通过 `ib_console` 编译；Build History 远程任务计数与缓存统计成为证明输入 |
| 受保护校验器 | 完整场景覆盖、按时序核对轨迹、产物身份 |
| 运行器脚本 | 临时 Git 工作区、全新输出、导出与清理 |

当前实现删除的是**工作区**，不是 Linux 主机。已验证的部署使用 EC2 主机。
**islo 是计划中的提供方**，现有脚本不会因为配置了 islo 密钥就启用远程调度。

决策门在每段运动前检查，一段运动可以包含多个仿真控制步。本演示不证明连续监督、
物理急停、硬件在环或机器人安全。250 ms 是示例策略，不是硬件安全阈值。

### Rust 机器人生态

[robotics.rs](https://robotics.rs/) 汇总 Rust 原生 ROS、仿真、规划、视觉与设备库。本演示刻意保持较小的外部表面：
安全契约、锁步宿主、证据模型、IB 遥测解析和证明门都使用 Rust；只有所选 robosuite 任务原生依赖 Python，
因此 Python 仅保留为仿真适配器。

[`nexus-robotics-os`](https://crates.io/crates/nexus-robotics-os) 4.2.0-rc.2
在能力／安全／证据理念上相容，但它仍是候选发布版，而且明确不声称 HIL 或实体验证。
我们不会只为品牌把它加入依赖：那会重复本演示的运行时并改变被测构建图。
未来的 Nexus 适配器应作为独立、可测量且有自己证据的集成。

## 历史证据能说明什么

运行 `ec2-e2e-20260923-160725` 包含 17 个仿真回合：
10 次举起、5 次陈旧拒绝、1 次仿真停止、1 次超时。
原修订报告了 10/10 项决策门测试和 88/88 项校验器检查，这些数字不代表当前加强后的测试套件。

构建 A 用时 **21.426 s**，B 用时 **21.948 s**，B **慢了 522 ms**。
两个记录均标明使用 IB，但不能据此证明辅助机执行、受控缓存状态、缓存复用或加速效果。
新指标明确标记缓存复用尚未核实，受控性能基准仍待运行。

历史可执行文件没有提交到仓库。摘要标识了记录中的产物，仅克隆日志无法独立核对该二进制。
新彩排会导出自己的可执行文件。[证据说明](docs/examples/README.md)

## 网站维护

中英文首页均为静态页面，标题与分享元数据随各自页面发布。
修改 `scripts/site/` 下的共享模板与翻译后执行：

```bash
python3 scripts/site/build.py
python3 scripts/site/build.py --check
python3 -m http.server 8765 --directory docs
```

离线可直接打开 `docs/index.html` 或 `docs/en/index.html`。
矩阵由已提交的结果生成，机器人视频默认暂停并带播放控件，交互回放支持手动逐步查看。

## 有意保留的缺陷

不要删除 `main` 中缺失时效检查的演示缺陷。应在验证流程中应用回退或候选补丁，
保留从失败到通过的完整演示。
