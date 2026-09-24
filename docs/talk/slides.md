---
marp: true
title: 你怎么知道？
paginate: true
style: |
  section { background: #10171d; color: #edf2f4; font-family: sans-serif; font-size: 28px; }
  h1, h2 { color: #edf2f4; }
  a { color: #f0b85d; }
  table { font-size: 24px; }
  code { color: #f0b85d; }
---

# 你怎么知道？

一个"成功"却全程错误的机器人，一道认证了四份伪造证明的证明门，以及唯一站住的那一级

[English](slides.en.md)

<!-- 本套幻灯片中的每个数字均于 2026-09-24 在同一台机器上实测。证据已提交：447 个文件。 -->

---

## 机器人成功了

![bg right:42% contain](../assets/robot-stale_600ms-poster.png)

`outcome=cube_lifted  success=true`
`dispatches=4  rejections=[]  wall=4088 ms`

每一盏灯都是绿的。

<!-- 画面来自运行 asset-stale-verify-1790261498，是陈旧观测回合，不是新鲜举起。robosuite 1.5.2 / mujoco 3.9.0。 -->

---

## 它全程都是错的

```
控制周期        |-20-|-20-|-20-|-20-|-20-|  ms
契约上限        |<----- 250 ms = 12.5 个周期 ----->|
它据以动作的    |<-------- 600 ms = 30 个周期 --------->|
```

观测年龄就在数据里。它照样下发了。

---

## 契约不同意

```
cargo test -p robot-safety-gate --test contract
  boundary_251ms_rejects ... FAILED
  stale_ages_are_rejected ... FAILED
  configured_threshold_is_respected ... FAILED
  5 passed; 3 failed
```

任务绿，契约红。**同一个二进制。**

---

## 第三条规则

```rust
// robot-safety-gate/src/lib.rs:147
let _ = (age_ms, policy);
```

一行代码。它消除了未使用变量的告警。

这就是全部的缺陷。

---

## 谁写的测试？

契约由三个测试锁定。

遗漏在第 147 行。

**同一个人。同一个下午。**

<!-- 这是第一级：检查的对手掌控着检查的输入。 -->

---

## 伪造一 —— 收据为自己背书

```
cache_cleared_before_each_cold_sample: true   <- 生产方自己写下的字面量

$ build-proof --receipt fabricated.json
BUILD PROOF PASS ... ratio=2.999x vs native; saved=14000ms      exit 0
```

八项失败即关闭条件中，有三项永远不可能触发。

---

## 伪造二 —— 六十四个零

```
transcript_path:   "/tmp/does-not-exist.txt"
transcript_sha256: "0000...0000"     <- 64 位十六进制。检查到此为止。

-> BUILD RECEIPT CONSISTENT ... ratio=11.948x                   exit 0
```

我们校验了摘要的形状。我们从未打开那个文件。

---

## 伪造三 —— 真文件，真摘要

十份记录由**我们自己的** `cache-clear.sh` 生成，包裹着一个三行脚本：它打印
`namespace user purged`，什么也不做。

十份全部打开。十份全部重新哈希。**"已佐证"。**

```
ratio=119.949x                                                  exit 0
```

<!-- 正是这一条改变了论点。可表达性不是那条轴。作者身份才是。 -->

---

## 伪造四 —— 我们自己的覆盖声明

```
coverage-matrix.json :  "freshness_ms": [0, 50, 600]
lib.rs:25            :  DEFAULT_MAX_OBSERVATION_AGE_MS = 250
```

十七个场景。校验器称之为**完整覆盖矩阵**。

任何超过 500 ms 的陈旧度都能通过全部十七个。

---

## 然后，我自己也犯了

```
-f, --force-remote   force allow_remote tasks to remote helpers
                     ^ ib-benchmark.sh:176

max_initiator_cores = 0      <- 四个本地核心。闲置。每一次运行。
```

我发布了"慢 2 倍"。我测的是四个远程核心**替换**四个闲置的本地核心。

---

## 我最得意的数字是错的

我推导出**87% 并行效率**，并围绕它做了一页幻灯片。

它用一台机器的 CPU 秒除以另一台机器的墙钟时间，再除以一个磁盘上并不存在的核心数。

**它感觉像洞见。** 这正是你自己炮制的论断从内部看起来的样子。

---

## 站住的部分

| | `-j1` | `-j10` | |
|---|---|---|---|
| 冷构建 | 22,861 ms | 6,976 ms | **3.28x** |
| 改一个文件后重建 | 1,016 ms | 942 ms | **1.08x** |

同一台机器，同一个工具，n=5。

真正要紧的那次重建是**串行的**。分布式卖的是并行度；这个工作负载已经没有并行度可卖。

---

## 一个词

```xml
<process filename="rustc" type="local_only">   <!-- 原为: allow_remote -->
  <ib_cache enabled="true" />
</process>
```

缓存与分布式是同一条声明上两个互相独立的开关。

---

## 两行都要给

| | cargo | 构建缓存 | |
|---|---|---|---|
| **空工作区** | 11,515 ms | **3,706 ms** | **3.13x** |
| 热工作区 | **939 ms** | 4,048 ms | cargo 更快 |

```
HIT 52 / MISS 0 · 58 个任务中只执行 9 个 · 辅助机 0
```

**缓存不会让你的构建更快。它让"扔掉工作区"变得便宜。**

---

## 提议者 : 认证者

```
策略   : 安全门      不持有 IO 句柄 —— 它"不能"驱动电机
候选   : 校验器      从轨迹重新推导 —— 无法被直接递给一个答案
任务   : 协调器      写入构建记录 —— 任务本身无法撰写
```

# 检查只能赢过那个无法掌控其输入的对手。

`rustc` 的对手无法撰写 `rustc`。我的校验器的对手，是我自己。

<!-- 结尾：我们造了一台捕捉未经验证论断的机器，把它对准自己的工作，它抓到我们四次。这是你应该相信这里任何数字的唯一理由。 -->
