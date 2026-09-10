# Cron Task

## Claude

```Question 1
定时任务如何配置和检查？

```

● Explore(Explore cron/scheduled task implementation)
  ⎿  Backgrounded agent (↓ to manage · ctrl+o to expand)

● 正在探索代码库中定时任务（cron）相关的实现，稍后汇报结果。

✻ Waiting for 1 background agent to finish

● Agent "Explore cron/scheduled task implementation" finished · 1m 46s

● ZeroClaw 定时任务（Cron）配置与检查机制

  先纠正一点：这个仓库实际是 Rust 项目（Cargo workspace），不是 Go。

  代码位置

  ┌──────────────┬──────────────────────────────────────────────────────────────────────┬─────────────────────────────────────┐
  │      层      │                                 路径                                 │                作用                 │
  ├──────────────┼──────────────────────────────────────────────────────────────────────┼─────────────────────────────────────┤
  │ CLI 命令     │ src/cron/mod.rs + src/lib.rs:323（CronCommands）                     │ zeroclaw cron ... 子命令            │
  ├──────────────┼──────────────────────────────────────────────────────────────────────┼─────────────────────────────────────┤
  │ 运行时核心   │ crates/zeroclaw-runtime/src/cron/{types,schedule,scheduler,store}.rs │ 数据结构、调度、执行、SQLite 持久化 │
  ├──────────────┼──────────────────────────────────────────────────────────────────────┼─────────────────────────────────────┤
  │ 配置 schema  │ crates/zeroclaw-config/src/schema.rs:6395-6528                       │ CronConfig/CronJobDecl              │
  ├──────────────┼──────────────────────────────────────────────────────────────────────┼─────────────────────────────────────┤
  │ REST API     │ crates/zeroclaw-gateway/src/{api.rs,lib.rs}                          │ /api/cron 系列端点                  │
  ├──────────────┼──────────────────────────────────────────────────────────────────────┼─────────────────────────────────────┤
  │ 守护进程接入 │ crates/zeroclaw-runtime/src/daemon/mod.rs:250-266                    │ 启动调度器协程                      │
  └──────────────┴──────────────────────────────────────────────────────────────────────┴─────────────────────────────────────┘

  如何配置

  两种方式，最终都落到同一个 SQLite 数据库 <workspace_dir>/cron/jobs.db：

  1. 命令式（CLI/REST 动态创建，source="imperative"）
  zeroclaw cron add '0 9 * * 1-5' 'Good morning' --tz America/New_York --agent
  zeroclaw cron add-at '2026-03-11T10:00:00Z' 'Remind me'
  zeroclaw cron add-every 3600000 'Check server health'
  zeroclaw cron once 30m 'Follow up'
  2. 声明式（写在 config.toml，source="declarative"，调度器每次启动时同步进数据库，不会覆盖命令式任务）
  [cron]
  enabled = true
  catch_up_on_startup = true
  max_run_history = 50

  [[cron.jobs]]
  id = "morning-report"
  job_type = "agent"
  enabled = true
  prompt = "Summarize overnight system health"

  [cron.jobs.schedule]
  kind = "cron"
  expr = "0 9 * * 1-5"
  tz = "America/New_York"

  调度类型（Schedule 枚举，三选一）：Cron{expr, tz}（标准 cron 表达式）、At{at}（一次性绝对时间）、Every{every_ms}（固定毫秒间隔）。

  任务体 job_type 分 shell（跑命令，超时 120s）和 agent（跑 prompt，可指定工具白名单/是否用记忆/独立或主会话）。

  如何检查/触发（运行机制）

  - 使用 Rust cron crate（非 Go 的 robfig/cron）解析表达式，配合 chrono-tz 处理时区。
  - 轮询而非精确定时器：scheduler::run（scheduler.rs:25-103）用 tokio::time::interval 按 config.reliability.scheduler_poll_secs（最低 5 秒）周期性查询 due_jobs（SQL：enabled=1 AND 
    next_run<=now），执行到期任务后重算并写回 next_run。
  - 启动时若 catch_up_on_startup=true，会一次性补跑所有已过期任务。
  - 执行结果可通过 delivery 配置推送到聊天渠道，历史记录存入 cron_runs 表（受 max_run_history 限制）。

  检查任务状态的方式

  zeroclaw cron list                    # 列出所有任务
  或 REST：GET /api/cron（列表）、GET /api/cron/{id}/runs（执行历史）、POST /api/cron/{id}/run（手动立即触发一次）、PATCH /api/cron/settings（改轮询设置）。

  注意：文档里另有一套 [[triggers]] type = "cron"（SOP 触发器，见 docs/book/src/sop/cookbook.md），是完全不同的机制，不要和这里的 cron 子系统混淆。

```Question 2
请分析介绍一下SOP 触发器 并比较两个机制有什么区别？                                                                                                                                         

```

● Explore(Explore SOP trigger mechanism)
  ⎿  Backgrounded agent (↓ to manage · ctrl+o to expand)

● 正在调研 SOP 触发器机制及其与 cron 任务调度器的架构差异，完成后汇报。

✻ Waiting for 1 background agent to finish

● Agent "Explore SOP trigger mechanism" finished · 2m 36s

● SOP 触发器机制分析

  什么是 SOP

  **SOP（Standard Operating Procedure，标准作业流程）**是一种"文件定义的多步骤工作流"，和 cron 任务（单次执行一条命令/prompt）本质不同。代码在
  crates/zeroclaw-runtime/src/sop/（audit、condition、dispatch、engine、metrics、types 子模块），CLI 壳在 src/sop/mod.rs。

  它解决的问题：把"如果发生 X，就按步骤 1、2、3 执行，中间可能需要人工审批"这种可重复的运维流程固化为一个版本化、可审计、可触发的工件——cron 任务没有多步骤结构和审批模型。

  数据模型（crates/zeroclaw-runtime/src/sop/types.rs）：
  - Sop（164-182 行）：name、priority、execution_mode（Auto/Supervised/StepByStep/PriorityBased/Deterministic）、triggers: Vec<SopTrigger>、steps: Vec<SopStep>、cooldown_secs、max_concurrent
  - 定义分两个文件：SOP.toml（元数据+触发器）+ SOP.md（## Steps 里的具体步骤，Markdown 格式，非 TOML）
  - 运行态 SopEngine（engine.rs:18-27）是纯内存结构（HashMap<String, SopRun>），没有专门的持久化数据库，重启即丢失活跃运行状态；只有审计日志经通用 Memory trait 写入（category "sop"）

  触发器类型

  crates/zeroclaw-runtime/src/sop/types.rs:69-88：
  #[serde(tag = "type", rename_all = "lowercase")]
  pub enum SopTrigger {
      Mqtt { topic: String, condition: Option<String> },
      Webhook { path: String },
      Cron { expression: String },
      Peripheral { board: String, signal: String, condition: Option<String> },
      Manual,
  }

  type = "cron" 的 TOML 写法（docs/book/src/sop/cookbook.md:77-79）：
  [[triggers]]
  type = "cron"
  expression = "0 9 * * *"

  SOP cron 触发器如何被检查（设计层面）

  crates/zeroclaw-runtime/src/sop/dispatch.rs：
  - SopCronCache（249-308行）：启动时把所有 SOP 的 cron 表达式解析并缓存成 cron::Schedule（复用同一个 cron crate，也复用了 job 调度器的 normalize_expression）
  - check_sop_cron_triggers()（314-346行）：给定 (last_check, now)，检查每个表达式在这个窗口内是否有一次触发点，命中则构造 SopEvent 交给统一入口 dispatch_sop_event()

  与 cron 任务调度器的结构对比

  ┌──────────────┬────────────────────────────────────────────────────────────────────────────┬───────────────────────────────────────────────────────────────────────────────────────────┐
  │     维度     │                              Cron 任务调度器                               │                                      SOP cron 触发器                                      │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ 配置位置     │ CronConfig，CLI/REST 动态管理                                              │ SOP.toml 的 [[triggers]]，随 SOP 定义加载                                                 │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ 持久化       │ SQLite（cron/store.rs），任务和执行历史重启后仍在                          │ 无专门持久化，仅内存 HashMap；只有审计日志写入通用 Memory                                 │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ 表达式解析   │ cron crate                                                                 │ 同一个 cron crate（代码复用）                                                             │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ 执行模型     │ tokio::time::interval                                                      │ 同样是窗口式轮询设计（(last_check, now]），但代码只在单元测试里被调用，没有被任何地方     │
  │              │ 真实轮询循环，作为守护进程子系统运行（daemon/mod.rs:250-260）              │ spawn 进 daemon                                                                           │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ 触发后做什么 │ 执行单条 shell 命令或一次 agent prompt                                     │ 启动一次完整的多步骤 SopRun，按 execution_mode 决定是全自动跑完还是每步暂停等人工审批     │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ 审批/门控    │ 无内置概念                                                                 │ 原生支持：Supervised/StepByStep + Checkpoint 步骤可暂停等待 sop_approve                   │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ 并发控制     │ 无对应机制                                                                 │ cooldown_secs + max_concurrent，由 SopEngine::can_start() 强制                            │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ 接入 daemon  │ ✅ 真实子系统                                                              │ ⚠️  只有 MQTT 来源是真接入的（main.rs:1805-1821），cron/webhook 触发器实现了但没被接线     │
  ├──────────────┼────────────────────────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
  │ REST API     │ 有（/api/cron/*）                                                          │ 完全没有，只有只读 CLI（sop list/validate/show）和 Agent                                  │
  │              │                                                                            │ 工具（sop_execute/sop_status/sop_approve/sop_advance）                                    │
  └──────────────┴────────────────────────────────────────────────────────────────────────────┴───────────────────────────────────────────────────────────────────────────────────────────┘

  一个值得注意的发现

  代码调研发现一个文档与实现不一致的地方：docs/book/src/sop/connectivity.md 描述了 cron 轮询检查和 webhook 路由（POST /sop/{*rest}）的"目标设计"，troubleshooting 表里甚至写着"Cron not firing
  — 检查 daemon 是否运行"，但实际在 crates/zeroclaw-gateway 和 daemon::run 里搜不到任何调用 check_sop_cron_triggers/dispatch_sop_event 的地方（除了单元测试）。也就是说：

  type = "cron" 和 type = "webhook" 的 SOP 触发器目前是"写了但没接线"的半成品，唯一真正端到端可用的自动触发方式是 type = "mqtt"，手动触发靠 sop_execute 工具。

  一句话总结

  - Cron 任务系统：轻量、单动作、SQLite 持久化、轮询已真实接入 daemon —— 是成熟功能。
  - SOP 触发器：面向多步骤工作流+审批门控的更高层设计，cron/webhook 触发路径已写好但当前版本未真正接入运行时，只有 mqtt 和手动调用是可用的。

