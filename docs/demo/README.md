# 舞台画面

`index.html` 是单页、自包含的安全门已记录裁决回放。用浏览器直接打开——无需服务器、无需构建，可离线（Noto Sans SC 与 IBM Plex Mono 随站点自托管）。默认简体中文，右上角可切 EN。

## 它展示什么

三个回合，各自从 `docs/examples` 里已提交的事件日志回放：

| 按钮 | 来源 | 要点 |
| --- | --- | --- |
| 陈旧回合 · 600 ms | `ec2-runner-b/events-center-f600.jsonl` | 补丁后门拒绝 600 ms 陈旧观测，什么都不派发。 |
| 新鲜回合 · 0 ms | `ec2-runner-b/events-center-f0.jsonl` | 同一构建在当前信息下允许全部四段并举起方块。 |
| 植入构建 · 同一陈旧输入 | `ec2-runner-a/events-stale_600ms.jsonl` | 植入构建在同一陈旧观测上允许全部四段并报告成功。 |

第三个值得停住：回合「通过了」。方块到 0.9985 m，仿真器报告 `success=true`。运行里看起来没有错，所以必须有受保护断言。

回放下方的矩阵是补丁构建的全部 17 回合——五个方块位置对三种观测年龄，外加急停与协议超时——证据条带带着校验器结果和做出这些裁决的可执行文件摘要。

## 怎么开

按 `1`、`2` 或 `3` 切换回合，不需要指针。幻灯也可以用 `index.html?play=seeded`（`stale`、`fresh`、`seeded`）直接链进去。

## 出处

每个 tick、观测年龄、裁决、派发计数和方块高度都从 run `ec2-e2e-20260923-160725` 抄录。要重新推导：

```sh
python3 - <<'PY'
import json
for l in open('docs/examples/ec2-runner-a/events-stale_600ms.jsonl'):
    e = json.loads(l); m = json.loads(e['line'])
    if m.get('type') == 'proposal':
        age = (m['simulation_time_ns'] - m['observation']['capture_time_ns']) // 10**6
        print(m['simulation_tick'], m['proposed_action']['segment'], f'{age}ms')
    elif e['dir'] == 'out':
        print('   ->', m.get('decision'), m.get('reason', ''))
PY
```

机械臂图是示意，不是 MuJoCo 帧截取——那次运行没有录像。页面上有标明。方块高度和全部裁决数据是真的。

页面故意不声称的一件事：两个构建阶段测得 21.4 s 和 21.9 s，说明加速路径两边都跑了。那不是加速测量，页面也这么写，而不是暗示基准。
