# 已提交的示例证据

> 这是原始版本的历史证据。88 项检查属于当时的校验器，不代表当前更严格校验器的新结果。归档不包含可执行文件；请用当前排练脚本重新生成完整证据。清理工作区不等于销毁 EC2；网格拓扑不证明辅助节点执行或缓存复用。


[English](README.en.md)

来自一次完整本地彩排（`scripts/robot-demo/rehearse.sh`，
`ROBOT_DEMO_BACKEND=robosuite`），CPython 3.12 / robosuite 1.5.2 /
MuJoCo 3.9.0，macOS arm64：

- `sil-runner-a/` — 植入修订的 **失败记录**：陈旧（600 ms）回合被错误允许（4 次任务派发），以及运行器销毁前导出的智能体上下文包。
- `sil-runner-b/` — **已核验的修复候选**：全新隔离运行器，把 `demo/fallback-patch.diff` 打在精确基线上，契约套件 10/10，17 回合覆盖矩阵，受保护校验器 **88/88 PASS**。

导出的可执行文件本身未提交（体积）；其 sha256 记在 `sil-runner-b/artifact/swf-cli.sha256` 和清单里。随时可用 `scripts/robot-demo/rehearse.sh` 再生成可核验产物。

范围提醒：这些是 Rust 契约检查、桥接检查与仿真机器人场景。未做硬件 HIL 与实体验证。

## Linux / Incredibuild 网格运行

`ec2-runner-a/` 与 `ec2-runner-b/` 来自 run
`ec2-e2e-20260923-160725`，Ubuntu 20.04 x86-64 Initiator 连到
Incredibuild 4.31.0 网格（一台 Coordinator 与两台 Helper）：

- 两个 Cargo 阶段都在 `ib_console` 下执行，使用已入库的 `rust/ib_profile.xml`；
- 运行器 A 复现植入缺陷：600 ms 陈旧回合派发了四次任务动作；
- 运行器 B 把审阅过的补丁打在精确基线修订上，通过全部 10 项 Rust 检查，并完成 17 回合 robosuite 矩阵；
- 受保护校验器对照导出的可执行文件身份通过 **88/88 项检查**。

观测到的构建阶段墙钟是冷 21.426 s、热 21.948 s。这证明 Incredibuild 集成路径，但 **不** 证明加速：热观测略慢。不要把这两次工作流连续性计时当成受控性能基准。计划中的同候选、每种模式至少五次的基准仍是另一次运行。
