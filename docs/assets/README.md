# 网站媒体

`robot-lift.gif` 含 robosuite 1.5.2 / MuJoCo 3.9.0 的真实帧。
回合经 `swf-cli` 跑过，所以每一段接近、下降、抓取、举起都先经 Rust 安全门授权，仿真器才执行。

从钉死的仿真环境重新生成：

```bash
scripts/robot-demo/record-gif.sh
```

脚本记录 `fresh_lift`，把得到的 MP4 转成 512×384、12 fps 的 GIF，并删掉临时证据目录。需要 `ffmpeg`；`gifsicle` 可选，用于再压缩。

这张 GIF 展示仿真运动。摘要绑定的会议证据仍是 `docs/examples/` 下已提交的 run `ec2-e2e-20260923-160725`；重新生成网站媒体不会替换那份证据。

## 闭环走查

网站播放的是 `validation-loop.mp4`（以及 `validation-loop-poster.png`）。`validation-loop.gif` 是同一段画面，留下来是因为 GitHub 的 README 渲染器不会播放提交的视频文件。闭环画面默认简体中文。

```bash
scripts/robot-demo/record-loop-gif.sh
```

三者都来自 `docs/demo/loop.html`，在 `?frame=N` 渲染确定状态，所以媒体可复现，而不是手拼。

站点用视频而不是 GIF，是为了无障碍，不是为了体积——MP4 实际上更大（约 417 KB 对 285 KB），因为 H.264 处理锐利终端文字不如 GIF 调色板。选它是因为走查有 19 秒：这么长的运动需要暂停控制，静音循环也不该对选择减少动态的访客自动开始。GIF 两样都做不到。

## 字体

`fonts/` 下是子集化的 Noto Sans SC（400/700）与 IBM Plex Mono（400/500），SIL Open Font License。见该目录 `LICENSE`。不要链 Google Fonts。
