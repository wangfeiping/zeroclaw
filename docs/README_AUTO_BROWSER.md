# Auto browser

## Config

```shell
# 需要确保可打开文件数足够，默认1024不够
ulimit -n
ulimit -n 65535

sudo vi /etc/security/limits.conf
* soft nofile 65535
* hard nofile 65535

# 安装agent-browser 和chrome
npm install -g agent-browser
agent-browser install --with-deps

### 正确参数调用

  agent-browser --profile "$HOME/.agent-browser-discord-profile" --args "--no-sandbox" --session discord \
    open https://discord.com/login

  *** 注意（如果没有显示器）：上面命令会显示成功信息，但没有打开浏览器（因为无头模式），需要登录并连接、导出cookies
  ***     需要安装虚拟显示

  # 1. 起一个带调试端口、已登录的 Chrome（后台运行）
  "$HOME/.agent-browser/browsers/chrome-153.0.8010.36/chrome" \
    --no-sandbox \
    --user-data-dir="$HOME/.agent-browser-discord-profile" \
    --remote-debugging-port=9222 \
    https://discord.com/login &
 
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

  DISPLAY=:99 google-chrome --no-sandbox --user-data-dir=$HOME/.agent-browser-discord-profile https://discord.com/login
```

## Question 1

```text
怎么配置启用 browser 工具？比如自动读取https://discord.com/channels/491685925227724801/999575963920781382 中的信息，我已登录。                                                              

```

Claude:

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

## Question 2

```text
请问浏览器自动化时是否支持同一个session多个网站？比如discord,X,linkin等等                                                                                                                   

```

Claude:

● 让我查一下ZeroClaw代码库中浏览器自动化相关的实现，来准确回答这个问题。

● Agent(Research browser automation session support)
  ⎿  Backgrounded agent (↓ to manage · ctrl+o to expand)

● 正在查找 ZeroClaw 代码库中浏览器自动化相关的实现，稍等。

✻ Waiting for 1 background agent to finish

● Agent "Research browser automation session support" finished · 38s

● 支持。在 ZeroClaw 里，浏览器自动化用的是 session_name（配置项 browser.session_name，对应 agent-browser --session <name>），本质上是一个持久化的 Chrome profile / 
  user-data-dir，不是一次对话的概念。

  这意味着：
  - 浏览器天然按域名隔离 cookie，所以同一个 session_name 下可以同时访问 discord.com、x.com、linkedin.com 等多个网站，各自独立保持登录状态，互不冲突——就像你平时用一个 Chrome
    窗口分别登录多个网站一样。
  - 该 profile 不会自动继承你日常 Chrome 的登录态，需要先用 VNC 手动登录一次各个网站；或者用 browser_delegate 工具直接复用本机真实 Chrome 的 profile（风险较高：会暴露该 profile
    下所有网站的登录信息，且和本机 Chrome 互斥使用）。
  - 访问域名受 allowed_domains/blocked_domains 白名单限制，需要显式把 discord.com / x.com / linkedin.com 都加进允许列表。
  - 高风险操作（比如发消息、点确认按钮）默认会走审批/OTP 网关（autonomy.require_approval_for_medium_risk、security.otp.gated_actions）。

  相关文件：
  - crates/zeroclaw-tools/src/browser.rs — 主浏览器工具
  - crates/zeroclaw-tools/src/browser_delegate.rs — 复用本机 Chrome profile
  - docs/book/src/tools/browser.md — 官方文档
  - docs/README_AUTO_BROWSER.md — 中文调试笔记，专门讨论过 Discord 登录态与 session 场景，跟你的问题高度对应，建议直接看一下

## Question 3

```text
就是说agent-browser永远不可能实际打开一个浏览器，必须通过别的方法创建并导出cookies才行是么？

```

Claude:

● 不完全对，需要澄清一下：

  agent-browser 本身是可以打开真实浏览器窗口的——加 --headed 参数就会显示交互式窗口（README 里明确写着 --headed | Show browser window on the interactive desktop），不是"永远不可能"。

  问题在于：远程服务器通常没有图形显示环境（没有 X11/Wayland/桌面）。--headed 依赖一个真实或虚拟的显示服务器才能弹出窗口——服务器上没有
  DISPLAY，这个参数自然用不了，你就没法用肉眼登录、过验证码/二次验证。这不是 agent-browser 的限制，是"远程无图形界面服务器"这个环境本身的限制。

  所以实际上有两条路，不是只有导出 cookie 一条：

  1. 导出 cookie 导入（你笔记里用的方法）：在有图形界面的机器（比如你本地电脑）上用真实 Chrome 交互登录一次，导出 cookie，传到服务器上导入到 --session discord 的 headless profile 里。
  2. 给服务器装一个虚拟显示环境（docs/book/src/tools/browser.md 里专门讲过）：装 Xvfb（虚拟帧缓冲）模拟一块"屏幕"，再配合 VNC 把这块虚拟屏幕转发出来，你就能远程"看到"并操作 --headed
     模式下真正弹出的浏览器窗口，在服务器本地完成交互登录（含验证码/2FA），不需要来回搬运 cookie。

  两种方法各有取舍：cookie 导入更轻量、不用额外装 Xvfb/VNC，但每次登录态失效都要重新走一遍导出导入；Xvfb+VNC
  装好后可以直接在服务器上交互操作，但环境搭建更重。你笔记里选的是前者，属于合理的工作方式选择，而不是 agent-browser 技术上做不到"打开真实浏览器"。

## Question 4

```text
远程服务器也有显示器

```

Claude:

● 如果服务器上确实接了显示器并且有人登录了图形桌面（不是纯 SSH 无头环境），那 --headed 大概率是可以直接用的，不需要绕过 cookie 导出这条路。关键要确认两点：

  1. 物理屏幕上是否真的有一个桌面会话在跑（有人登录了 GNOME/KDE/XFCE 等，而不是屏幕接着但一直停在登录界面/黑屏）——这决定了背后是否存在一个活的 X11/Wayland server。
  2. 你执行 agent-browser 命令的那个 shell 能不能连上那个显示会话——如果你是通过 SSH 远程登录去敲命令，SSH session 默认不会自动拿到物理控制台那个桌面的 DISPLAY，需要手动指定：

  # 先在服务器本地（或已知桌面会话）确认显示编号和权限
  echo $DISPLAY          # 桌面会话本身跑起来后，在它自己的终端里执行能看到，比如 :0 或 :1
  who                     # 看谁登录了图形会话
  loginctl show-session $(loginctl | awk '/tty|seat/{print $1; exit}') -p Type 2>/dev/null

  # 在 SSH 里手动指定同一个 DISPLAY 并给权限
  xhost +local:           # 在物理桌面会话的终端里执行一次，允许本地连接
  export DISPLAY=:0       # 在 SSH 会话里设置成和桌面会话一致的编号
  export XAUTHORITY=/home/<桌面登录用户>/.Xauthority   # 通常需要这个才能鉴权成功

  # 测试能不能连上
  xdotool getactivewindow 2>&1   # 或者装了 xrandr 的话: xrandr

  确认这几步通了之后，直接加 --headed 试：

  agent-browser --profile "$HOME/.agent-browser-discord-profile" --args "--no-sandbox" \
      --session discord --headed \
      open https://discord.com/login

  如果这样能看到浏览器窗口真的弹在那块屏幕上，你就可以直接在服务器上完成交互登录（含验证码/2FA），不再需要"本地登录导出 cookie
  再导入"这套流程了——那套流程本来就是为纯无头、没有任何显示手段的服务器准备的备用方案，不是必经之路。

  如果 --headed 试了没反应或报连接显示服务器失败，把具体报错发我，我再帮你继续查是权限问题还是压根没有桌面会话在跑。

## Question 5

```text
只有显卡是否可以？必须是物理上的显示器设备并且登录并保持打开常亮的正常显示状态？如果是远程登录的图形界面呢？是否可以？                                                                      

```

Claude:

● 这几个问题分开答：

  只有显卡够不够？

  不够，也不是必需的。 显卡（GPU）只是硬件加速渲染用的，跟"是否存在一个显示环境"是两回事。真正的门槛是：有没有一个正在运行、且你能连上的显示服务器进程（X server 或 Wayland
  compositor）。反过来说，就算完全没有显卡（纯 CPU 服务器），只要跑一个虚拟显示服务器（比如 Xvfb），Chrome 照样能用软件渲染（SwiftShader）连上去正常工作——这也是无头 Chrome
  本身默认的工作方式。所以有 GPU 没有显示服务器 = 不行；没 GPU 但有虚拟显示服务器 = 完全可以。

  必须是物理显示器、登录、保持常亮显示吗？

  都不是必须的。

  - 物理显示器：不需要。Xvfb（虚拟帧缓冲）创建的是一块纯内存里的"假屏幕"，物理上根本没有接显示器这回事，Chrome 照样能对着它渲染。
  - 必须登录：需要的是"有一个活着的桌面会话/X server 进程"，不是"必须有人用眼睛看着"。如果是真实物理机的图形桌面，通常只要用户登录过、会话没退出（哪怕锁屏了），X server 进程还在跑，DISPLAY
    依然有效——锁屏一般只是上面盖了一层锁屏应用，不等于把 X session 关掉了。
  - 保持常亮显示：屏幕因为节能进入黑屏/关闭背光，也不影响 X server 存在与否，DISPLAY 通常照样能用。真正会失效的是"用户完全注销/X server 进程被杀掉了"这种情况。

  远程登录的图形界面（RDP/VNC/NoMachine 之类）可以吗？

  完全可以，而且这才是服务器场景下的标准做法。 这类远程桌面工具背后本质上也是在维护一个 X（或 Wayland）会话：

  - VNC（TigerVNC/x11vnc）：vncserver 本身就是启动一个类似 Xvfb 的虚拟 X 显示（比如 :1），只是额外把画面通过 VNC 协议转发出去让你能看、能操作。这是最推荐的方案，因为搭建轻量、跟
    agent-browser 的需求完全匹配。
  - xrdp / NoMachine：通常是共享或新建一个真实的桌面会话，原理类似，同样有一个可连接的 DISPLAY。

  具体做法（服务器完全没有物理显示器也适用）：

  # 装一个轻量虚拟显示 + 轻量窗口管理器
  sudo apt install tigervnc-standalone-server fluxbox

  # 启动一个虚拟显示（首次会让你设 VNC 密码）
  vncserver :1 -geometry 1280x800 -depth 24

  # 让 agent-browser 用这个显示
  export DISPLAY=:1
  agent-browser --profile "$HOME/.agent-browser-discord-profile" --args "--no-sandbox" \
      --session discord --headed \
      open https://discord.com/login

  然后在你自己的电脑上用任意 VNC 客户端连 服务器IP:5901，就能看到并操作这个刚弹出来的浏览器窗口，直接在服务器本地完成登录（含验证码/2FA）。登录一次后，因为你用的是持久化 --profile
  目录，登录态会留在那个 profile 里，后续无头运行时直接复用，不需要再搬运 cookie。

  一句话总结：决定能不能用 --headed 的是"有没有一个可连接的显示服务器"，跟有没有物理显示器、有没有独立显卡、有没有人正盯着屏幕看都无关——远程 VNC/RDP
  图形界面完全满足条件，甚至比物理显示器更方便（不用真去机房插屏幕）。


