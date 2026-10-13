# ComfyUI skill

## 安装

```text
docs/comfyui/skill/ 下:

  - SKILL.md:skill 本体,带 /comfyui 斜杠命令,参数是 prompt(必填)、type(image 或 video)、duration、aspect_ratio。
  - README.md:部署、测试和排错说明,比下面更详细。

  skill 没有脚本,只固化调用规则,针对的是这次踩过的坑:

  - 只用 Partner 模型:图片默认 grok,视频默认 grok-video;禁用 stability-* 和本地工作流。
  - 直接调 comfyui__ 工具:不许用 mcp_resources 判断是否挂载,不许走 docker exec。
  - 参数放在 params 里:输出写到 agent 自己 workspace 的 comfy_output/。
  - 每次请求只扣一次费:最多重试一次,超过 15 秒的视频或多个文件要先确认。
  - 回复带附件标记:[IMAGE:路径] 或 [VIDEO:路径],Discord 会作为附件上传。
  
  部署(以 devbot 为例)

  cd /opt/gopath/src/github.com/wangfeiping/zeroclaw
  mkdir -p ~/.zeroclaw/agents/devbot/workspace/skills
  cp -r docs/comfyui/skill ~/.zeroclaw/agents/devbot/workspace/skills/comfyui
  zeroclaw skills audit ~/.zeroclaw/agents/devbot/workspace/skills/comfyui
  zeroclaw skills list --agent devbot      # 应出现 comfyui
  systemctl --user restart zeroclaw

  容器挂载和 MCP 配置之前已经做好,不需要再动。

  测试

  先用命令行,它是全新会话,最容易排查:

  # 不扣费
  zeroclaw agent -a devbot -m "调用工具 comfyui__server_info,告诉我 ComfyUI 版本号"

  # 图片,约几美分
  zeroclaw agent -a devbot -m "用 comfyui skill 生成一张图:一只橘猫坐在窗台上"

  # 5 秒视频,约 $0.10
  zeroclaw agent -a devbot -m "用 comfyui skill 生成一段 5 秒视频:一只橘猫在窗台上慢慢转头看向窗外"

  ls -lt ~/.zeroclaw/agents/devbot/workspace/comfy_output | head

  然后在 Discord 里:

  /comfyui prompt:一只橘猫坐在窗台上
  /comfyui prompt:一只橘猫在窗台上慢慢转头看向窗外 type:Video duration:5

  还没验证的地方

  - 成功结果里的文件路径:到目前为止没有一次 partner_generate 成功过,所以我不知道它的返回里路径字段长什么样。skill 里写的是"从结果读路径,读不到就取 comfy_output/ 里最新的文件"。
  - 视频的 resolution 参数:480p 是按 README 价格表写的,schema 没有列可选值;skill 里规定报错时去掉它重试一次。
  - 斜杠命令的格式:frontmatter 是照 skills/tts 的写法,审计通过了,但 /comfyui 是否在 Discord 里正常注册要装上后才知道。

  第一次生成成功后把结果(或报错)贴给我,我按实际返回把 skill 里这几处改准。
```

## 测试

```text
图片和视频各跑了一次,都成功了,skill 里之前靠猜的几处已按实际返回改好并重新通过审计。

  实测结果

  ┌──────┬───────────────────────────┬──────────┬──────────┬───────────────────────────────────────┐
  │      │           模型            │   耗时   │   费用   │               输出文件                │
  ├──────┼───────────────────────────┼──────────┼──────────┼───────────────────────────────────────┤
  │ 图片 │ grok                      │ 约 11 秒 │ 约 $0.02 │ comfy_output/1ce063d1_0.jpg(350 KB)   │
  ├──────┼───────────────────────────┼──────────┼──────────┼───────────────────────────────────────┤
  │ 视频 │ grok-video,5 秒,16:9,480p │ 约 33 秒 │ 约 $0.25 │ comfy_output/cbd680b1-…_0.mp4(883 KB) │
  └──────┴───────────────────────────┴──────────┴──────────┴───────────────────────────────────────┘

  文件在 ~/.zeroclaw/agents/devbot/workspace/comfy_output/。费用是按返回里的 cost_in_usd_ticks 换算的(按 1 美元 = 10¹⁰ ticks)。账户余额从 $25.00 降到了 $24.42,其中包含你在 Discord
  里生成的那几次。

  视频实际约 $0.25,比 README 价格表里估的 $0.10 高;docs/comfyui/README.md 的价格表我没有动。

  返回结果的格式

  两种调用的返回结构一致,下载到本地的文件路径在 saved 数组里:

  {
    "result": { "...": "厂商原始返回,含临时签名 url 和 usage.cost_in_usd_ticks" },
    "saved": ["/home/mil2er/.zeroclaw/agents/devbot/workspace/comfy_output/1ce063d1_0.jpg"]
  }
```

## 修改

```text
skill 的改动
 
  - 取文件路径:改为直接用 saved[0],并明确不要用 result 里的 url(那是临时签名链接)。
  - resolution:480p 实测可用,去掉了"报错就去掉重试"的规则,改为默认保持 480p。
  - 费用说明:SKILL.md 和 README.md 里都换成了实测数字。
   
  skill 还没有安装到 devbot(它的 workspace 下没有 skills/ 目录),这次是我绕过 agent 直接调 comfy-mcp 测的。安装命令照旧:

  cd /opt/gopath/src/github.com/wangfeiping/zeroclaw
  mkdir -p ~/.zeroclaw/agents/devbot/workspace/skills
  cp -r docs/comfyui/skill ~/.zeroclaw/agents/devbot/workspace/skills/comfyui
  zeroclaw skills list --agent devbot
  systemctl --user restart zeroclaw

  装好后 /comfyui 斜杠命令和 [IMAGE:…]/[VIDEO:…] 附件回传这两步还需要你在 Discord 里试一次,这部分我没法从本地验证。
```

