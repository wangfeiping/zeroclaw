# comfyui skill — 部署与测试

让 ZeroClaw agent 通过 ComfyUI 的 Partner 模型生成图片或 5 秒左右的短视频,并把文件作为附件发回。
Discord 用户执行 `/comfyui prompt:<描述>`,或者直接 @bot 用自然语言提出。

```
Discord /comfyui ──► ZeroClaw agent ──MCP/stdio──► docker exec -i comfyui comfy-mcp
                          │                              │ comfy-cli (COMFY_API_KEY)
                          │                              ▼
                          │                        api.comfy.org (Partner 模型)
                          │                              │
                          │        <workspace>/comfy_output/<文件>  ◄── 下载
                          ▼
reply "[IMAGE:/…/comfy_output/….png]" ──► Discord 作为附件上传
```

skill 本身只有 `SKILL.md`,没有脚本。它固化的是调用规则:只用 Partner 模型、参数放在
`params` 里、输出写到 agent 自己的 workspace、每次请求只扣一次费。

## 1. 前置条件

以下三项在 `docs/comfyui/README.md` 的流程里已经完成,这里只列检查命令。

```sh
# 容器在运行,且带有 key
docker exec comfyui sh -c 'test -n "$COMFY_API_KEY" && echo key-ok'

# agent 的 workspace 以相同的绝对路径挂进了容器(否则生成的文件 agent 读不到)
docker inspect comfyui --format '{{json .HostConfig.Binds}}'

# agent 被授权使用 comfyui 这个 MCP 服务
zeroclaw config get agents.<alias>.mcp-bundles      # 应包含 "comfy"
```

对应的 `~/.zeroclaw/config.toml` 片段:

```toml
[[mcp.servers]]
name = "comfyui"
command = "docker"
args = ["exec", "-i", "comfyui", "comfy-mcp"]
tool_timeout_secs = 600

[mcp_bundles.comfy]
servers = ["comfyui"]

[agents.<alias>]
mcp_bundles = ["comfy"]
```

容器启动命令(每个要用这个 skill 的 agent 都要有一行 workspace 挂载):

```sh
docker run -d --name comfyui --restart unless-stopped \
   --env-file /opt/ai/comfy/.env \
   -p 127.0.0.1:8188:8188 \
   -v /data/comfy/output:/app/ComfyUI/output \
   -v /data/comfy/user:/app/ComfyUI/user \
   -v /home/<user>/.zeroclaw/agents/<alias>/workspace:/home/<user>/.zeroclaw/agents/<alias>/workspace \
   comfyui-max:latest
```

## 2. 安装 skill

把本目录复制到 agent 的 workspace(把 `<alias>` 换成 agent 名,例如 `devbot`):

```sh
cd /opt/gopath/src/github.com/wangfeiping/zeroclaw
mkdir -p ~/.zeroclaw/agents/<alias>/workspace/skills
cp -r docs/comfyui/skill ~/.zeroclaw/agents/<alias>/workspace/skills/comfyui
zeroclaw skills audit ~/.zeroclaw/agents/<alias>/workspace/skills/comfyui
zeroclaw skills list --agent <alias>        # 应出现 comfyui
```

然后重启 daemon,让 skill 和 `/comfyui` 斜杠命令生效:

```sh
systemctl --user restart zeroclaw
```

Discord 的斜杠命令在配置了 `guild_ids` 时立即注册,否则全局注册可能要等一段时间。

## 3. 测试

每一步生成都会扣费。实测(2026-10-13):`grok` 图片约 $0.02、耗时约 11 秒;`grok-video` 5 秒 480p 约 $0.25、耗时约 33 秒。

命令行(全新会话,最容易排查):

```sh
# 不扣费:确认 agent 能直接调用 MCP 工具
zeroclaw agent -a <alias> -m "调用工具 comfyui__server_info,告诉我 ComfyUI 版本号"

# 图片,约 $0.02
zeroclaw agent -a <alias> -m "用 comfyui skill 生成一张图:一只橘猫坐在窗台上"

# 5 秒视频,约 $0.25
zeroclaw agent -a <alias> -m "用 comfyui skill 生成一段 5 秒视频:一只橘猫在窗台上慢慢转头看向窗外"

ls -lt ~/.zeroclaw/agents/<alias>/workspace/comfy_output | head
```

Discord:

```
/comfyui prompt:一只橘猫坐在窗台上
/comfyui prompt:一只橘猫在窗台上慢慢转头看向窗外 type:Video duration:5
@bot 帮我画一张图:雨夜的东京街头,霓虹灯倒映在积水里
```

## 4. 排错

| 现象 | 原因和处理 |
|---|---|
| agent 说没有 ComfyUI 工具 | 先用上面不扣费的命令行测试。命令行可以而 Discord 不行,是频道会话里的旧历史在误导模型:在频道里单独发 `/new`。命令行也不行,检查 `mcp_bundles` 并重启 daemon。 |
| `workflow_not_found` 或找不到输出文件 | 该 agent 的 workspace 没有挂进容器,按第 1 节重建容器,再重启 daemon。 |
| `API error 404` | 所选模型的接口已下线(例如 `stability-sd3`),换用 `grok`、`recraft` 或 `flux-2`。 |
| `API error 401/402` | key 无效或余额不足,到 platform.comfy.org 检查。 |
| 视频调用超时 | MCP 单次调用上限是 600 秒。任务可能已经提交并扣费,不要立刻重试。 |
| 回复里的附件被拒绝(🚫) | 标记里的路径必须是 agent 自己 workspace 内的绝对路径。 |

查看日志:

```sh
docker logs --tail 50 comfyui
grep -E "MCP|comfyui__" ~/.zeroclaw/data/state/runtime-trace.jsonl | tail
```

## 5. 注意

- 频道里所有能触发 bot 的人都能花掉 Comfy 账户的额度。需要限制时,只把 skill 装给特定 agent,
  或在 Discord 侧限制 `/comfyui` 命令的可用角色。
- 容器以 root 运行,`comfy_output/` 里的文件属主是 root,agent 可读,手工删除需要 `sudo`。
- `comfy_output/` 不会自动清理,需要的话加一条 cron,例如:
  `find ~/.zeroclaw/agents/*/workspace/comfy_output -mtime +7 -delete`(属主为 root 时用 root 的 crontab)。
- 默认模型写在 `SKILL.md` 的 Models 表里。换模型前先执行
  `docker exec comfyui comfy generate schema <model>` 看参数。
