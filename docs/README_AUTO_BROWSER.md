# Auto browser

```shell
ulimit -n
ulimit -n 65535

DISPLAY=:99 google-chrome --no-sandbox --user-data-dir=$HOME/.agent-browser-discord-profile https://discord.com/login

### 正确参数调用
agent-browser --profile "$HOME/.agent-browser-discord-profile" --args "--no-sandbox" --session discord                                                                                      
  open https://discord.com/channels/491685925227724801/999575963920781382

  # 1. 起一个带调试端口、已登录的 Chrome（后台运行）
  "$HOME/.agent-browser/browsers/chrome-153.0.8010.36/chrome" \
    --no-sandbox \
    --user-data-dir="$HOME/.agent-browser-discord-profile" \
    --remote-debugging-port=9222 \
    https://discord.com/channels/491685925227724801/999575963920781382 &
 
  sleep 2
  
  # 2. agent-browser 连上它，导出 cookie
  agent-browser connect 9222
  agent-browser cookies get --json > /tmp/discord-cookies.json
  cat /tmp/discord-cookies.json   # 看看导出了多少条，确认不是空的

  # 3. 关掉这个临时 Chrome
  kill %1

  # 4. 切回 agent-browser 自己 headless 的 discord session，导入 cookie
  agent-browser close --all
  agent-browser open --session discord https://discord.com
  agent-browser cookies set --curl /tmp/discord-cookies.json --domain discord.com --session discord

  # 5. 验证
  agent-browser open --session discord https://discord.com/channels/491685925227724801/999575963920781382
  agent-browser screenshot /tmp/discord-check2.png --session discord

  把第 2 步 cat 的内容（条数/是否有 discord.com 相关 cookie，比如 token 之类的名字，具体值不用贴）和第 5 步截图结果告诉我。

```

## Claude

怎么配置启用 browser 工具？比如自动读取https://discord.com/channels/491685925227724801/999575963920781382 中的信息，我已登录。                                                              

  Searched for 6 patterns, read 3 files, ran 2 shell commands 

● 启用步骤如下（配置文件通常在 ~/.zeroclaw/config.toml，或项目里的 config.toml）：
 
  1. 启用 browser 工具
 
  [browser]
  enabled = true
  allowed_domains = ["discord.com"]   # 限定域名，别用 "*"
  backend = "agent_browser"           # 默认后端，无需改
  session_name = "discord"            # 关键：给这次自动化会话起名，用来保存登录态

  或用 CLI：
  zeroclaw config set browser.enabled true
  zeroclaw config set browser.allowed-domains '["discord.com"]'
  zeroclaw config set browser.session-name discord

  2. 安装依赖

  npm install -g agent-browser
  agent-browser install --with-deps   # 拉取 Chrome for Testing

  3. 关键问题：登录态怎么复用

  agent_browser 后端跑的是独立的无头浏览器 profile，不会自动继承你日常 Chrome 里的 Discord 登录态。要让它"已登录"，有两种选择：

  方式 A（推荐，风险低）：借助 VNC 手动登录一次这个自动化专用的 session
  # 起一个有头浏览器 + VNC（见 docs/book/src/tools/browser.md）
  agent-browser open --session discord https://discord.com/login
  # 通过 VNC 图形界面手动完成登录（含 2FA）
  登录一次后，cookie 会保存在 --session discord 对应的 profile 里，之后 ZeroClaw 用同一个 session_name = "discord" 调 agent-browser 时会自动复用这个已登录状态，不需要每次都登录。

  方式 B：用 browser_delegate 工具直接复用你本机真实 Chrome 的用户数据目录
  [browser_delegate]
  enabled = true
  chrome_profile_dir = "/home/你的用户名/.config/google-chrome"
  allowed_domains = ["discord.com"]
  这样能直接拿到你现有的登录态，但要注意：Chrome 的 profile 目录是互斥锁的，自动化脚本运行时你自己那个 Chrome 窗口不能同时开着；而且这等于把你整个 Chrome
  数据（所有网站的登录态）暴露给自动化工具，安全上要谨慎，不建议长期这么用。

  4. 权限/审批

  项目默认把 browser 放进了需要审批甚至 OTP 网关的动作列表（autonomy.require_approval_for_medium_risk、security.otp.gated_actions），如果是个人本地场景想免确认，需要相应调整
  autonomy.auto_approve，但这会降低安全性，按需取舍。

  5. 测试

  echo "打开 https://discord.com/channels/491685925227724801/999575963920781382，读取并总结最新消息" | zeroclaw agent

  需要注意：自动化访问 Discord 属于第三方网站的自动化行为，是否符合 Discord 服务条款需要你自行评估；另外这个频道信息可能含隐私内容，建议先在受控环境测试。

