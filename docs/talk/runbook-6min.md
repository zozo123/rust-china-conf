# 六分钟舞台手册

[English](runbook-6min.en.md) · [讲稿](talk-25min.md)

## 会前准备

提交准备演示的源码。运行器脚本使用已提交的修订；
提交前可用 `python3 scripts/robot-demo/check.py` 检查当前改动。

在 Linux 发起机上选择真实仿真并要求使用 IB：

```bash
export REQUIRE_IB=1
export ROBOT_DEMO_BACKEND=robosuite
export IB_ALLOW_CLEAR_USER_CACHE=1
export IB_HISTORY_URL='https://<coordinator>:8000/api/builds?coordinatorId=<id>&version=1.5.0'
export IB_CLIENT_API_KEY='<仅保存在本机的密钥>'
export ROBOT_DEMO_PYTHON="$PWD/demo/robot-sim/.venv/bin/python3"
scripts/robot-demo/preflight.sh
scripts/robot-demo/ec2-agentic-physical-ai.sh rehearsal-unique-id
```

该流程只清空当前用户的本地 IB 缓存。保留原始 Build History 与缓存遥测、
Rust 证明收据、完整 17 场景证据、可执行文件和校验报告。确认仿真器可初始化且结果满足契约。
完成三次不超过 5:30 的计时彩排，脚本打印的阶段时间并非实际耗时。

Mac 上保留本地网站、媒体及完整且标明为录像的演示。
历史回放入口为 `docs/demo/index.html?play=seeded&lang=zh`。

## 现场顺序

| 时间 | 动作 | 讲解 |
| --- | --- | --- |
| 0:00–0:45 | 植入缺陷版本举起方块，同时展示两个结论 | “方块举起来了，时效契约失败了。” |
| 0:45–1:30 | 审查补丁；如已配置，可尝试外部智能体 | “候选改动不能重新定义验收标准。” |
| 1:30–2:00 | 导出补丁和基线，展示 A 工作区已删除 | “保留改动和证据，删除工作区。” |
| 2:00–3:00 | B 使用记录的基线和全新输出构建 | “构建这个候选，再重新执行检查。” |
| 3:00–4:30 | 修复后拒绝陈旧请求，再完成新鲜观测任务 | “陈旧派发为零，新鲜任务仍能完成。” |
| 4:30–5:30 | 展示证据摘要、范围与实际计时 | “这些结果属于这个文件和这次运行。” |
| 5:30–6:00 | 应急余量 | 必要时切换到明确标注的录像。 |

如果现场只运行两个精选场景，必须将完整 17 场景矩阵标为**彩排覆盖**，不能说是刚刚完成的实时结果。

## 回放操作

- **1：** 陈旧输入下的植入缺陷版本。
- **2：** 陈旧输入下的修复版本。
- **3：** 新鲜输入下的修复版本。
- **空格：** 焦点在页面时播放／暂停。
- **右箭头：** 下一条决策，最后一步显示回合结论。
- **R：** 重置到暂停的起点。
- **显示结果：** 直接显示最终结论。

机械臂姿态是示意，事件数据来自记录。真实仿真画面使用首页中单独标注的视频。

## 分阶段执行

使用唯一编号，脚本会拒绝复用已有证据目录。

```bash
scripts/robot-demo/cold.sh stage-a
# cold.sh 返回前已导出证据并删除 A。
# 此处可运行外部智能体，等待上限约 45 秒。
# 否则明确选用已审阅的回退补丁：
export ROBOT_DEMO_PATCH_FILE="$PWD/demo/fallback-patch.diff"
export ROBOT_DEMO_BASE_REVISION="$(cat evidence/stage-a/agent-context/base-revision.txt)"
scripts/robot-demo/warm.sh stage-b
scripts/robot-demo/validate.sh stage-b
```

默认流程使用已审阅的补丁，不调用大语言模型。白名单在应用前检查候选补丁。
B 执行完整矩阵；若时间不足，展示已经完成且明确标注的彩排，不要把少量实时用例说成完整测试。

## 准确表述

- 当前运行器是 EC2 Linux 主机上的临时工作区，删除工作区不会销毁主机。islo 接入仍在计划中。
- 新旧运行编号不同，检查数量也可能不同。历史 10/10 和 88/88 属于归档修订。
- 历史构建观测为 21.426 s 与 21.948 s，B 慢 522 ms；它们仍是未受控的历史数据。
- 新实验针对同一候选，分别执行原生、IB 冷缓存、IB 父修订预热构建，每种至少五个样本。
  只有 Rust 校验器通过远程任务、远程核时与缓存命中检查，且收据支持结论时，
  才能声称加速或缓存复用。
- 本演示是按运动段授权的软件在环，不涉及实体验证或逐控制步的连续监督。
