# 网站媒体

首页使用由原有 `robot-lift.gif` 转换的 `robot-lift.mp4` 与 `robot-lift-poster.png`。画面来自 robosuite / MuJoCo 的新鲜观测举起回合，不是实时机器人画面，也不是陈旧观测回合。播放由用户控制。

在固定版本的 Linux 环境中运行 `scripts/robot-demo/record-gif.sh` 可重新录制。网站媒体不替代 `docs/examples/` 中与摘要绑定的证据。

`validation-loop*` 视频、GIF 与海报是旧版走查素材。其历史缓存与运行器措辞不能证明缓存复用、辅助节点参与或虚拟机销毁。保留文件供旧链接使用；演讲请使用当前首页与交互回放。

`robot-stale_600ms.{gif,mp4,png}` 是陈旧观测回合——演讲开场使用的画面。录自运行
`asset-stale-verify-1790261498`，其 `scenario-results.json` 记录为
`scenario: stale_600ms · outcome: cube_lifted · success: true · rejections: []`
（robosuite 1.5.2 / mujoco 3.9.0）。任务成功，新鲜度契约失败。
使用 `scripts/robot-demo/record-scenario-gif.sh` 配合 `SCENARIO=stale_600ms` 可重新录制。
与 `robot-lift.*` 不同，这段画面展示的正是演讲要讲的失败。
