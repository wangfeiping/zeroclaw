# 快速指导

## 配置

zeroclaw quickstart               # 交互式配置向导
zeroclaw doctor                   # 检查配置是否正确

sudo npm install -g agent-browser # 自动化浏览器插件
zeroclaw service install          # systemd / OpenRC
zeroclaw service start

### 或

systemctl --user status zeroclaw

### 需要特别关注的配置项

vi ~/.zeroclaw/config.toml

#temperature = 0.7 默认值需要修改
temperature = 1
api_key = "enc2:***"
model = "kimi-code-2.5"

## 启动命令行对话

zeroclaw agent -a max

## Error

### 部署配置时没有报错，但运行对话时报错。

```text
  kimi.com/code/console 申请的 Kimi for Coding 专用 Key，只能打 https://api.kimi.com/coding/v1，跟普通 Moonshot 聊天 API（.cn/.ai）不是同一套后端，混用必然 401。

  最终配置（~/.zeroclaw/config.toml）：
  [providers.models.moonshot.kimi]
  api_key = "enc2:..."
  model = "k3"
  endpoint = "code"
  context_window = 128000

  - endpoint = "code" → 路由到 https://api.kimi.com/coding/v1
  - model = "k3" → 你选的通用旗舰模型（该 Key 下还有 k3-256k、kimi-for-coding、kimi-for-coding-highspeed 可选，回复里已列出取舍）
  - 补了 context_window = 128000，消掉了 doctor 之前那条"用 32000 token 兜底"的警告，跟你 runtime_profile 里 max_context_tokens = 128000 对齐
```

### agent-browser 访问chatgpt.com测试总是被Cloudflare拦截

  ┌───────────────────────┬─────────────────────────────────────────────────────────┬───────────────────────────────────────┐
  │                       │                          之前                           │                 现在                  │
  ├───────────────────────┼─────────────────────────────────────────────────────────┼───────────────────────────────────────┤
  │ headless 指纹         │ webdriver=true、UA 含 HeadlessChrome、WebGL=SwiftShader │ 已消除（AGENT_BROWSER_HEADED=1 生效） │
  ├───────────────────────┼─────────────────────────────────────────────────────────┼───────────────────────────────────────┤
  │ 访问 chatgpt.com 结果 │ 卡在 Cloudflare "Just a moment..." 验证页               │ 正常加载到 ChatGPT 首页               │
  ├───────────────────────┼─────────────────────────────────────────────────────────┼───────────────────────────────────────┤
  │ "未登录"判断          │ 需要人工登录或不同session                               │ 正常：登录并使用正确的session         │
  └───────────────────────┴─────────────────────────────────────────────────────────┴───────────────────────────────────────┘

## .zeroclaw/config.toml 

```toml
### 注意配置部分
###
### 一、连接可访问AI模型
### [providers.models.moonshot.kimi]
### api_key = "api_key加密密文 enc2:..."
### model = "k3"
### endpoint = "code"
### context_window = 128000
### 
### 二、连接channel: discord 等访问控制
### [peer_groups.owner]
### channel = "discord"
### external_peers = ["纯数字user_id"]

schema_version = 3

[providers]

[providers.models]

[providers.models.moonshot]

[providers.models.moonshot.kimi]
api_key = "api_key加密密文 enc2:..."
model = "k3"
endpoint = "code"
context_window = 128000

[channels]

[channels.discord]

[channels.discord.discord]
bot_token = "bot_token加密密文 enc2:..."
enabled = true

[peer_groups.owner]                                                                                                                                                         
channel = "discord"                                                                                                                                                         
external_peers = ["纯数字user_id"]

[agents]

[agents.max]
model_provider = "moonshot.kimi"
risk_profile = "yolo"
runtime_profile = "unbounded"
channels = ["discord.discord"]

[memory]
backend = "sqlite.sqlite"

[runtime_profiles]

[runtime_profiles.unbounded]
agentic = true
agentic_timeout_secs = 1800
compact_context = false
delegation_timeout_secs = 900
keep_tool_context_turns = 8
max_actions_per_hour = 4294967295
max_context_tokens = 128000
max_cost_per_day_cents = 4294967295
max_delegation_depth = 8
max_history_messages = 200
max_system_prompt_chars = 64000
max_tool_iterations = 100
max_tool_result_chars = 64000
memory_recall_limit = 10
parallel_tools = true
shell_timeout_secs = 600
strict_tool_parsing = false

[risk_profiles.yolo]
allowed_commands = ["*"]
auto_approve = ["*"]
block_high_risk_commands = false
level = "full"
require_approval_for_medium_risk = false
sandbox_enabled = false
workspace_only = false

[runtime_profiles.unbounded.context_compression]
enabled = false
identifier_policy = "strict"
max_passes = 3
protect_first_n = 3
protect_last_n = 4
source_max_chars = 50000
summary_max_chars = 4000
threshold_ratio = 0.5
timeout_secs = 60
tool_result_retrim_chars = 2000

[risk_profiles.yolo.delegation_policy]
mode = "allow"

[runtime_profiles.unbounded.eval]
enabled = false
max_retries = 1
min_quality_score = 0.5

[runtime_profiles.unbounded.history_pruning]
collapse_tool_results = true
enabled = false
keep_recent = 4
max_tokens = 8192

[runtime_profiles.unbounded.thinking]
default_level = "medium"
display = "off"
native_thinking = false

[runtime_profiles.unbounded.tool_receipts]
enabled = false
inject_system_prompt = true
show_in_response = false

[onboard_state]
quickstart_completed = true

[risk_profiles]
```

## zeroclaw.service

```shell
[Unit]
Description=ZeroClaw daemon
After=network.target

[Service]
Type=simple
ExecStart=/home/mil2er/.cargo/bin/zeroclaw daemon
Restart=always
RestartSec=3
# Ensure HOME is set so headless browsers can create profile/cache dirs.
Environment=HOME=%h
Environment="CHROMIUM_FLAGS=--no-sandbox --disable-dev-shm-usage"
Environment=AGENT_BROWSER_HEADED=1
# 让 daemon 永不因为空闲而自动退出，预热一次之后就一直活着
Environment=AGENT_BROWSER_IDLE_TIMEOUT_MS=0
# 服务启动后，用完全正确的交互式方式预热一次 daemon
ExecStartPost=/bin/sh -c 'sleep 2; /home/mil2er/.nvm/versions/node/v24.13.1/bin/agent-browser open || true'
# Allow inheriting DISPLAY and XDG_RUNTIME_DIR from the user session
# so graphical/headless browsers can function correctly.
PassEnvironment=DISPLAY XDG_RUNTIME_DIR

[Install]
WantedBy=default.target
```

### agent-browser

```shell
DISPLAY=:99 agent-browser \
    --profile "$HOME/.agent-browser-discord-profile" \
    --args "--no-sandbox" \
    --session discord \
    --headed \
    open https://chatgpt.com

agent-browser close --all

pkill -f agent-browser
```

