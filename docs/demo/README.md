# 安全门的已记录回放

[English](README.en.md)

`index.html` 提出一个问题：**任务完成了吗？新鲜度契约通过了吗？** 这是两个独立的结果。方块可以被举起，而安全门同时违反其策略。

直接用浏览器打开页面即可，无需构建、服务器、机器人连接或网络。字体与已记录数据均随站点提供。默认简体中文，右上角可切换 **EN**，也可使用 `?lang=en`。

## 按顺序比较

回放首先展示植入缺陷的构建，然后比较相同陈旧输入下的修复后构建，最后检查新鲜输入：

| 按键 / 回合 | 任务完成？ | 新鲜度契约通过？ | 已提交事件日志 |
| --- | --- | --- | --- |
| **1 · 植入缺陷 · 600 ms** | **是** — 4 次派发，方块到 0.9985 m | **失败** — 观测年龄超过 250 ms 策略 | [`ec2-runner-a/events-stale_600ms.jsonl`](../examples/ec2-runner-a/events-stale_600ms.jsonl) |
| **2 · 修复后 · 600 ms** | **否** — 机械臂保持，0 次派发 | **通过** — 拒绝陈旧输入 | [`ec2-runner-b/events-center-f600.jsonl`](../examples/ec2-runner-b/events-center-f600.jsonl) |
| **3 · 修复后 · 0 ms** | **是** — 4 次派发，方块到 0.9978 m | **通过** — 使用当前观测 | [`ec2-runner-b/events-center-f0.jsonl`](../examples/ec2-runner-b/events-center-f0.jsonl) |

第一个回合在 tick 65 以 `success=true` 结束，却一直使用 600 ms 前的观测。这正是受保护断言必须独立于仿真器任务成功标志的原因。修复后的陈旧回合在 tick 12 结束；新鲜回合在 tick 54 结束。

下方表格展示修复后构建的全部 17 个已记录回合：五个方块位置 × 三种观测年龄，加上急停与超时。证据条带描述该次已记录的修复后构建，包括校验器结果与可执行文件摘要；打开页面不会重新运行仿真器。

## 演示与检查

- **1 / 2 / 3** 按上表顺序选择回合。
- **空格** 在页面获得焦点时播放或暂停；按钮获得焦点时，空格保留正常的按钮操作。
- **右箭头** 暂停并显示下一个已记录裁决；最后再按一次显示回合结果。
- **R** 将当前回合重置到暂停的起点。
- **查看结果** 立即显示全部裁决与两个最终结果。
- 切换浏览器标签页会暂停播放。启用“减少动态效果”时，每个回合默认暂停，仍可播放或逐步检查。
- 切换语言会保留当前回合与播放位置，并翻译当前裁决、控制和轨迹说明。

可直接链接到某个回合，例如 [`index.html?play=seeded&lang=zh`](index.html?play=seeded&lang=zh)。`play` 接受 `seeded`、`stale` 或 `fresh`，无效值默认选择植入缺陷回合。选择回合会更新链接，不会重新加载页面。

## 哪些是真实记录，哪些是示意

tick、观测年龄、安全门裁决、派发计数、回合结果和最终方块高度来自 run `ec2-e2e-20260923-160725`。协议面板是可读摘要，来源链接打开原始 JSONL。机械臂姿态和播放节奏是示意，**不是已记录的关节坐标或 MuJoCo 录像**；那次运行没有录像。

在仓库根目录可检查原始消息：

```sh
python3 - <<'PY'
import json
from pathlib import Path
path = Path('docs/examples/ec2-runner-a/events-stale_600ms.jsonl')
for line in path.read_text().splitlines():
    event = json.loads(line)
    message = json.loads(event['line'])
    if message.get('type') == 'proposal':
        age = (message['simulation_time_ns']
               - message['observation']['capture_time_ns']) // 10**6
        print(message['simulation_tick'],
              message['proposed_action']['segment'], f'{age} ms')
    elif event['dir'] == 'out':
        print('  ->', message.get('decision'), message.get('reason', ''))
    elif message.get('type') == 'episode_end':
        print(message)
PY
```

250 ms 策略是演示值，不是实体机器人的安全限值。仿真不能替代硬件验证。两个构建阶段测得 21.4 s 与 21.9 s；它们说明 Incredibuild 构建路径已运行，不是加速测量。
