# 开发报告：本地模型 Provider 并发限流（`max_concurrent`）

版本: v0.7.5-a（dev-v0.7.5 分支）　日期: 2026-07-22

---

## 一、背景与需求

ZeroClaw 默认对 AI Provider 的调用是**有限并发**的：多渠道/多会话之间会并发发起 LLM
请求（受渠道分发信号量限制，上限 8~64，按渠道数动态计算），只有同一会话内部的多轮对话
是串行的。

这个模型对云端 API（Anthropic、OpenAI 等）没问题，但当 Provider 指向**本地/自建模型
服务器**（Ollama、llama.cpp、vLLM 等）时就会出问题——本地服务器的显存/算力往往扛不住
多个并发推理请求，一旦被并发打过去，轻则排队爆内存，重则直接崩溃或 OOM。

本次改动为 `ModelProviderConfig` 增加了一个可选的并发上限开关，让用户可以按 Provider
配置文件（profile）粒度限制"同时打到这个 Provider 的请求数"，同时不影响其他 Provider
以及渠道分发层、会话队列层原有的并发行为。

---

## 二、修改内容

### 1. `crates/zeroclaw-config/src/schema.rs`

`ModelProviderConfig` 新增字段：

```rust
/// Maximum number of concurrent in-flight requests allowed against this
/// provider. Useful for self-hosted/local model servers (Ollama,
/// llama.cpp, vLLM) that don't have the hardware to serve multiple
/// inference requests at once — set this to `1` to serialize all calls
/// to that server. Leave unset for cloud providers, which handle their
/// own concurrency.
pub max_concurrent: Option<usize>,
```

不设置（默认 `None`）= 行为不变，不限流。

### 2. `crates/zeroclaw-providers/src/semaphored.rs`（新文件）

新增 `SemaphoredProvider`：一个包装任意 `Provider` trait 实现的薄壳，内部持有一个
`tokio::sync::Semaphore`。所有会触发网络请求的 trait 方法（`chat_with_system` /
`chat_with_history` / `chat` / `chat_with_tools` / `list_models` / `warmup`，以及三个
流式方法）在真正发出请求前都会先获取一个信号量许可，许可数即为配置的并发上限。

- 流式方法：许可在后台任务里获取并持续持有到整条流结束，避免调用方拿到 stream 但不消费
  导致许可永久占用。
- `max_concurrent = 0` 会被当作 `1` 处理，防止把 Provider 意外锁死为完全不可用。
- 其余不涉及网络 I/O 的元数据方法（`capabilities`、`default_temperature`、
  `convert_tools` 等）直接透传给内部 Provider，不经过信号量。

### 3. `crates/zeroclaw-providers/src/lib.rs`

- `ProviderRuntimeOptions` 新增 `max_concurrent: Option<usize>` 字段，走和
  `native_tools`、`provider_timeout_secs` 完全一样的传递路径：
  `provider_runtime_options_from_config()` 从当前生效的 Provider profile 里读取
  `max_concurrent` 并塞进去。
- `create_provider_with_url_and_options()`（Provider 工厂函数，也是所有具体 Provider
  被 `Box::new` 装箱的唯一出口）在返回前新增一步：如果 `options.max_concurrent` 有值，
  就用 `SemaphoredProvider` 包一层再返回。

  这个函数同时被"主 Provider"和"fallback Provider"两条创建路径复用（见
  `create_resilient_provider_with_options`），所以限流对两条路径都生效，不会出现
  fallback 绕过限流的情况。

### 4. `CHANGELOG-next.md`

在 `### Providers` 小节补充了一条变更说明。

---

## 三、配置使用说明

在 `config.toml` 里给目标 Provider profile 加一行 `max_concurrent`：

```toml
[providers.models.ollama]
base_url = "http://localhost:11434"
max_concurrent = 1        # 同一时间最多放行 1 个请求打到本地 Ollama

# 其他本地/自建 Provider 同理，例如：
[providers.models.vllm]
base_url = "http://localhost:8000/v1"
max_concurrent = 2        # 本地显卡能扛 2 路并发就设 2
```

也可以用 CLI 直接设置（沿用现有的 `Configurable` 派生宏，自动生效，无需额外接线）：

```bash
zeroclaw config set providers.models.ollama.max-concurrent 1
```

不配置这个字段的 Provider（例如云端 Anthropic/OpenAI）行为完全不受影响，继续按渠道分发
信号量的默认上限并发。

---

## 四、已知限制

`ProviderRuntimeOptions` 目前只从"当前生效的 fallback/default Provider profile"构建
一次，再传给任意 Provider 的创建调用——这是项目里 `provider_timeout_secs`、
`native_tools` 等字段已有的既定行为模式，不是本次改动引入的新问题。

也就是说：如果配置了 `model_routes` 让不同请求路由到多个不同 Provider，`max_concurrent`
目前只会准确对应"当前激活/fallback"的那个 profile。对于"本地模型作为主力或唯一
fallback"这种典型场景是够用的；如果之后要支持"每条路由各自独立设置
`max_concurrent`"，需要额外改造 `provider_runtime_options_from_config`，可以后续单独
处理。

---

## 五、测试验证

新增单元测试（`crates/zeroclaw-providers/src/semaphored.rs`）：

- `caps_concurrent_chat_calls`：并发发起 5 个请求，验证任意时刻同时在跑的请求数不超过
  配置的上限。
- `zero_max_concurrent_is_treated_as_one`：验证 `max_concurrent = 0` 被安全地当作 `1`
  处理。

全量回归测试结果（`cargo clean` 后从零重新编译）：

| Crate | 结果 |
|---|---|
| zeroclaw-providers | 809 passed, 0 failed |
| zeroclaw-channels | 1224 passed, 0 failed |
| zeroclaw-config | 620 passed, 0 failed |
| zeroclaw-gateway | 171 passed, 0 failed |
| zeroclaw-runtime | 1622 passed, 0 failed |
| zeroclawlabs（主二进制 + acp-bridge） | 237 + 236 + 18 passed, 0 failed |

`cargo fmt --check` 通过。`apps/tauri`（桌面端）未纳入验证——该 crate 在本机环境本来就
因缺少系统库 `libsoup-3.0` 无法编译，与本次改动无关。

---

## 六、变更文件清单

```
 CHANGELOG-next.md                       |  4 ++++
 crates/zeroclaw-config/src/schema.rs    |  8 ++++++++
 crates/zeroclaw-providers/src/lib.rs    | 19 +++++++++++++++++--
 crates/zeroclaw-providers/src/semaphored.rs (新增)
```
