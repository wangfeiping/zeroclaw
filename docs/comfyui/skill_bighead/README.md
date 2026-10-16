# bighead skill — 名画大头照

用户发一张自拍和一张世界名画,agent 用 ComfyUI 的 Partner 模型 `grok-edit`(xAI Grok Imagine 图片编辑)
把自拍的脸换进名画,按名画的画风重绘,并把头部放大,生成一张"名画大头照"发回。

```
Discord @bot + 两张图 ──► ZeroClaw 把附件存到 <workspace>/discord_files/,消息里给出 [IMAGE:路径]
                               │
   1. shell: python3 skills/bighead/scripts/upload.py <自拍> <名画>
                               │  docker cp → 容器内转成 JPEG(长边 ≤ 2048)
                               │  → comfy generate upload → Comfy 存储的签名 URL(24 小时有效,不扣费)
                               ▼
   2. MCP: comfyui__partner_generate model=grok-edit
           params.images=[{url:自拍}, {url:名画}],提示词里用 <IMAGE_0>/<IMAGE_1> 指代
                               │  扣费一次
                               ▼
   <workspace>/comfy_output/<文件>.jpg ──► 回复 "[IMAGE:路径]" ──► Discord 作为附件上传
```

## 为什么需要上传脚本

`grok-edit` 的 `images` 参数是一组 `{url}` 对象,只接受公网 URL 或 base64 data URI。
comfy-cli 只会给最外层的单个字段自动上传本地文件,`images` 不在其中;而 comfy-mcp 每个参数
上限约 128 KB,直接塞 base64 放不下一张正常照片。comfy-mcp 也没有暴露 `comfy generate upload`
(`upload` 是被拒绝的保留词)。所以由 `scripts/upload.py` 在容器里调用 `comfy generate upload`
拿到 URL,再交给 `partner_generate`。

脚本用 Python 而不是 shell:ZeroClaw 的 skill 审计默认拦截 `.sh` 文件(`skills.allow_scripts = false`)。
脚本只用标准库,图片转换用容器里 ComfyUI 自带的 Pillow,宿主机不需要装额外的包。

## 1. 前置条件

与 `docs/comfyui/skill/README.md` 相同,另加一条:

- 容器 `comfyui` 在运行且带有 `COMFY_API_KEY`;
- agent 绑定了 `comfy` 这个 MCP bundle;
- agent 的 workspace 以相同绝对路径挂进了容器(`partner_generate` 的输出要写回 workspace);
- **新增**:运行 zeroclaw 的用户能直接执行 `docker`(在 docker 组里)。上传脚本通过 `docker cp` /
  `docker exec` 工作,所以输入图片不需要挂载进容器。

```sh
docker exec comfyui sh -c 'test -n "$COMFY_API_KEY" && echo key-ok'
docker ps --format '{{.Names}}' | grep -x comfyui      # 当前用户能用 docker
```

容器名不是 `comfyui` 时,在 agent 的环境里设置 `COMFY_CONTAINER=<名字>`。

## 2. 安装(以 devbot 为例)

```sh
cd /opt/gopath/src/github.com/wangfeiping/zeroclaw
mkdir -p ~/.zeroclaw/agents/devbot/workspace/skills
cp -r docs/comfyui/skill_bighead ~/.zeroclaw/agents/devbot/workspace/skills/bighead
zeroclaw skills audit ~/.zeroclaw/agents/devbot/workspace/skills/bighead
zeroclaw skills list --agent devbot      # 应出现 bighead
systemctl --user restart zeroclaw
```

目录名必须是 `bighead`:SKILL.md 里的脚本路径写的是 `skills/bighead/scripts/upload.py`。

## 3. 测试

先单独测上传脚本(不扣费):

```sh
# small images
# ┌───────┬─────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
  │ 宽度  │ 大  │                                                                                  链接                                                                                   │
  │       │ 小  │                                                                                                                                                                         │
  ├───────┼─────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ 1280p │ 约  │ https://upload.wikimedia.org/wikipedia/commons/thumb/e/ec/Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from_C2RMF_retouched.jpg/1280px-Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_fro │
  │ x     │ 957 │ m_C2RMF_retouched.jpg                                                                                                                                                   │
  │       │  KB │                                                                                                                                                                         │
  ├───────┼─────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │       │ 约  │ https://upload.wikimedia.org/wikipedia/commons/thumb/e/ec/Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from_C2RMF_retouched.jpg/960px-Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from │
  │ 960px │ 505 │ _C2RMF_retouched.jpg                                                                                                                                                    │
  │       │  KB │                                                                                                                                                                         │
  ├───────┼─────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │       │ 约  │ https://upload.wikimedia.org/wikipedia/commons/thumb/e/ec/Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from_C2RMF_retouched.jpg/500px-Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from │
  │ 500px │ 115 │ _C2RMF_retouched.jpg                                                                                                                                                    │
  │       │  KB │                                                                                                                                                                         │
  └───────┴─────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
#
#
# https://upload.wikimedia.org/wikipedia/commons/thumb/e/ec/Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from_C2RMF_retouched.jpg/500px-Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from_C2RMF_retouched.jpg
#

cd ~/.zeroclaw/agents/devbot/workspace
python3 skills/bighead/scripts/upload.py /path/to/selfie.jpg \
  "https://upload.wikimedia.org/wikipedia/commons/e/ec/Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from_C2RMF_retouched.jpg"
# IMAGE_URL_0=https://storage.googleapis.com/...
# IMAGE_URL_1=https://storage.googleapis.com/...
```

再用命令行跑完整流程(扣费一次):

```sh
zeroclaw agent -a devbot -m "用 bighead skill 做名画大头照:自拍 [IMAGE:/path/to/selfie.jpg],名画 https://upload.wikimedia.org/wikipedia/commons/e/ec/Mona_Lisa%2C_by_Leonardo_da_Vinci%2C_from_C2RMF_retouched.jpg ,头放大 2 倍"
ls -lt ~/.zeroclaw/agents/devbot/workspace/comfy_output | head
```

Discord 里:

- **推荐**:在一条消息里 @bot,附上两张图(第一张自拍,第二张名画),写"做名画大头照"。
- 斜杠命令 `/bighead` 不能带附件。只有自拍已经在之前的消息里发过、名画用 `painting:` 填图片链接时才能直接用;
  否则 agent 会提示用户改用带附件的普通消息。
- 选项:`scale`(1.5x / 2x / 3x,默认 2x)、`aspect_ratio`(默认 3:4)、`note`(额外要求,例如"保留眼镜")。

## 4. 费用

| 步骤 | 费用 |
|---|---|
| 上传两张图到 Comfy 存储 | 不扣费 |
| `grok-edit` 1k 一张 | 约 $0.02–0.05(未实测;文生图 `grok` 实测约 $0.02) |

skill 规定:每次请求只生成一张;API 报错最多同参数重试一次;内容被拒不重试;不经用户同意不换更贵的模型。

## 5. 排错

| 现象 | 原因与处理 |
|---|---|
| `BIGHEAD_ERROR=image N: cannot read image (...)` | 不是 Pillow 能读的图片(常见是 iPhone 的 HEIC),改发 JPG/PNG |
| `BIGHEAD_ERROR=... download failed (HTTP Error 403/400)` | 图片站拒绝下载(防盗链或缩略图尺寸不合法),换一个直链或直接发图片附件 |
| `BIGHEAD_ERROR=... container comfyui is not running` / `no such object` | 容器没启动或名字不对,检查 `docker ps`,必要时设置 `COMFY_CONTAINER` |
| `BIGHEAD_ERROR=... permission denied ... docker.sock` | 运行 zeroclaw 的用户不在 docker 组 |
| `partner_generate` 报 `images` 参数格式错误 | agent 把数组写成了字符串;SKILL.md 已要求传 JSON 数组,可在消息里再强调一次 |
| 生成成功但长相不像 / 头没放大 | 模型效果问题,调 SKILL.md 第 3 步的提示词模板,或加 `note` |

## 6. 隐私与使用限制

- 自拍会上传到 Comfy 的存储(Google Cloud 上的签名 URL,24 小时有效),再发送给 xAI 处理。
- skill 要求第二张图必须是艺术作品,遇到真人照片之间换脸会拒绝;也会拒绝用于嘲弄、色情化或冒充他人的请求。

## 7. 未验证的地方

- **没有真实跑过一次 `grok-edit`**:提示词效果、实际扣费、返回里文件路径字段是 `saved` 还是 `saved_paths` 都待第一次生成确认。
- `aspect_ratio` 是否会被 xAI 接受:接口说明写的是"多图编辑时设置",按此默认 3:4;报错时可去掉。
- 已验证:上传脚本对本地 PNG(3000×1000)、维基百科原图 URL(约 90 MB 的蒙娜丽莎)都能拿到 URL;
  非图片文件、不存在的文件、无法解析的域名、容器名错误都返回 `BIGHEAD_ERROR`;容器内临时文件会清理;
  `zeroclaw skills audit` 通过。

## Error

### Discord 中没有 /bighead 命令

```text
# ~/.zeroclaw/config.toml 里的 [channels.discord.discord] 需要开启 slash_commands。

  - 这个开关默认是 false，关着的时候 ZeroClaw 一个斜杠命令都不注册，不只是缺 /bighead，
  - 连内置的 /ask 也没有；
 
# 修复
 
  在 ~/.zeroclaw/config.toml 的 [channels.discord.discord] 下面加三行：

  [channels.discord.discord]
  enabled = true
  bot_token = "..."
  mention_only = true
  slash_commands = true             # 开启斜杠命令
  slash_command_scope = "guild"     # 只注册到指定服务器，马上生效
  guild_ids = ["<你的服务器 ID>"]   # 填你的服务器 ID

  - 服务器 ID 怎么拿： 在 Discord 设置 → 高级里打开"开发者模式"，然后右键服务器图标 →"复制服务器 ID"。
  - 能不能不填 guild_ids： 可以，不填会注册成全局命令，但全局命令最多要约 1 小时才会出现。测试的时候用 guild 更方便。

```

