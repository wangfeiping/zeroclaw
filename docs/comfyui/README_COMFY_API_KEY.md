# 注册申请和配置COMFY_API_KEY

让 ZeroClaw 通过 comfy-mcp 调用 ComfyUI 生成图片和视频i

* Docker 镜像已构建好，key 未配置；
* 注册 platform.comfy.org；
* 创建 API key 并充值；
* key 写入 /opt/ai/comfy/.env。

```test
● 镜像和 key 的读取路径已经核对完,再确认一下 ZeroClaw 这边 MCP 配置的字段格式就给你完整步骤。

  Ran 1 shell command 

● 镜像 comfyui-max:0.39.1 已经构建好,但还没有容器在运行,ZeroClaw 的 config.toml 里也还没有任何 MCP 配置。下面的步骤我没有实际执行,只核对了镜像里的代码和 ZeroClaw 的配置结构。

  1. 注册并申请 key
 
  1. 打开 https://platform.comfy.org 注册并登录。
  2. 进入 API Keys 页面创建 key。key 以 comfyui- 开头,只在创建时显示一次,要当场保存。
  3. 在同一站点的账户页面购买额度(credits);按 README 的估算,$5 足够初步测试。

  API Keys 页面的具体地址我没有登录确认:README 里写的是 /profile/api-keys,comfy-cli 的提示里写的是 /api-keys,以登录后的菜单为准。

  2. 把 key 写入 env 文件

  sudo mkdir -p /opt/ai/comfy /data/comfy/output /data/comfy/user
  sudo sh -c 'umask 077; echo "COMFY_API_KEY=comfyui-你的key" > /opt/ai/comfy/.env'
 
  等号两边不要加空格,值也不要加引号,否则 --env-file 会把引号当成 key 的一部分。

  3. 启动容器

  docker run -d --name comfyui --restart unless-stopped \
     --env-file /opt/ai/comfy/.env \
     -p 127.0.0.1:8188:8188 \
     -v /data/comfy/output:/app/ComfyUI/output \
     -v /data/comfy/user:/app/ComfyUI/user \
     comfyui-max:latest
     
  build_docker_comfyui_max.sh 末尾注释里的镜像名是 comfyui-comfymcp,而实际构建出来的是 comfyui-max,照抄注释会找不到镜像。
     
  检查 key 是否进了容器(不会打印 key 本身):
 
  docker exec comfyui sh -c 'test -n "$COMFY_API_KEY" && echo ok'

  *** agent 把工作流文件写到了宿主机的 ~/.zeroclaw/agents/max/workspace/,再把这个路径交给容器里的 comfy-mcp,而容器看不到这个目录。

  把这个目录按相同的绝对路径挂了进去,其余参数不变。重建后服务已启动,key 还在,容器里也能读到刚才那个 orange_cat_workflow.json。以后要重建容器请用这条:

  docker run -d --name comfyui --restart unless-stopped \
     --env-file /opt/ai/comfy/.env \
     -p 127.0.0.1:8188:8188 \
     -v /data/comfy/output:/app/ComfyUI/output \
     -v /data/comfy/user:/app/ComfyUI/user \
     -v /home/mil2er/.zeroclaw/agents/max/workspace:/home/mil2er/.zeroclaw/agents/max/workspace \
     comfyui-max:latest

  *** 不同agent 会使用不同路径
  docker run -d --name comfyui --restart unless-stopped \
     --env-file /opt/ai/comfy/.env \
     -p 127.0.0.1:8188:8188 \
     -v /data/comfy/output:/app/ComfyUI/output \
     -v /data/comfy/user:/app/ComfyUI/user \
     -v /home/mil2er/.zeroclaw/agents/devbot/workspace:/home/mil2er/.zeroclaw/agents/devbot/workspace \
     -v /home/mil2er/.zeroclaw/agents/max/workspace:/home/mil2er/.zeroclaw/agents/max/workspace \
     comfyui-max:latest

  4. key 是怎么生效的

  ComfyUI 服务本身不读 COMFY_API_KEY。读它的是 comfy-cli(comfy-mcp 调用它),提交工作流时把 key 放进请求体的 extra_data.api_key_comfy_org,Partner 节点从那里取。

  所以 key 必须在 comfy-mcp 进程的环境里。用 docker exec 启动 comfy-mcp 时会继承容器的环境变量,第 3 步的 --env-file 就够了,不需要把 key 再写进 ZeroClaw 配置。

  注意不要在容器里执行 comfy cloud login:登录会话的优先级高于 COMFY_API_KEY,会改用登录账户扣费。

  5. 在 ZeroClaw 里注册 MCP 服务

  在 ~/.zeroclaw/config.toml 里加:

  [[mcp.servers]]
  name = "comfyui"
  command = "docker"
  args = ["exec", "-i", "comfyui", "comfy-mcp"]
  tool_timeout_secs = 600

  [mcp_bundles.comfy]
  servers = ["comfyui"]
  
  [agents.max]
  mcp_bundles = ["comfy"]

  - bundle 必须配:没有 mcp_bundles 的 agent 不会连接任何 MCP 服务。你已有 [agents.max] 和 [agents.devbot],把 mcp_bundles 这一行加到现有的段里,不要重复建段。
  - 超时上限是 600 秒:视频生成仍然要用 run_workflow(wait=False) 提交后轮询,不能同步等。
  - 重启会话才生效:bundle 的改动只在会话或 agent 重建时读取。

  配好 key 并启动容器后,我可以帮你把这段配置写进 config.toml,再用最便宜的档位跑一张图验证整条链路(会产生几美分的扣费)。

```

## 测试

```text
*** 按顺序执行,前两步不扣费。

  1. 确认 agent 能看到 ComfyUI 工具

  zeroclaw agent -a max -m "调用 comfyui 的 server_info 和 auth_status 工具,把结果原样告诉我"
 
  2. 看有哪些 Partner 模型可用

  zeroclaw agent -a max -m "调用 comfyui 的 list_partner_models,列出最便宜的 5 个图片模型"

  3. 生成一张图(会扣费,约几美分以内)
 
  zeroclaw agent -a max -m "用 comfyui 选一个最便宜的图片模型,生成一张 1024x1024 的图:一只橘猫坐在窗台上。完成后告诉我输出文件名"

  生成视频：
  zeroclaw agent -a devbot -m '调用 comfyui__partner_generate,model=grok-video,params={"prompt":"一只橘猫、一只柴犬和一只鸭子并排坐在窗台上,橘猫慢慢转头看向窗外,温暖的午后阳光","duration":5,
  "aspect_ratio":"16:9","resolution":"480p"},confirm_spend=true,out_path=/home/mil2er/.zeroclaw/agents/devbot/workspace/,timeout_seconds=600。我已同意这次扣费。完成后告诉我输出文件名'

  4. 查看结果

  ls -lt /data/comfy/output | head

  这个目录是按上一轮的 docker run 挂载参数写的;如果你启动容器时用了别的路径,换成对应目录。

  出问题时

  - agent 说没有 comfyui 工具:加 -v 重跑第 1 步看 MCP 连接日志。
  - Discord 侧不生效:后台的 zeroclaw daemon(PID 2273)还在用旧配置,要重启它才会加载;只用上面的 zeroclaw agent 命令测试则不需要。
  - 第 3 步报缺少凭证或 401:执行 docker exec comfyui sh -c 'test -n "$COMFY_API_KEY" && echo ok',有输出说明 key 在,那就去 platform.comfy.org 检查 key 是否有效、额度是否到账。
  - 看 ComfyUI 侧日志:docker logs --tail 50 comfyui。

*** agent 选错了生成方式(需要改提示词)
     
  路径通了以后,那个工作流校验仍然不过:agent 写的是本地 SD1.5 工作流(CheckpointLoaderSimple 加 v1-5-pruned-emaonly.safetensors)。这个容器是纯 CPU、没有装任何模型,只能走 Partner 模型。
  
  提示词里要明确指定用 partner_generate:
  
  zeroclaw agent -a max -m "用 comfyui 的 MCP 工具生成一张图,不要写本地工作流(容器里没有任何本地模型)。步骤:1) list_partner_models 找最便宜的图片模型;2) partner_model_schema 查它的参数;3) 
  partner_generate 生成:一只橘猫坐在窗台上,1024x1024,confirm_spend=true,out_path=/home/mil2er/.zeroclaw/agents/max/workspace/。我已同意这次扣费。完成后告诉我输出文件名"

*** 
  - 余额非零:接口返回 amount_cents: 2500,应该是 $25,具体数额以 platform.comfy.org 页面为准。
  - 原因在模型目录:comfy-cli 1.22.0 自带的目录里仍然列着 stability-sd3,但服务端已经不提供,所以"选最便宜的"会选到一个不可用的模型。

  新的测试命令

  直接指定一个确认可达的模型。用 grok(README 里的估算约 $0.02 一张):
                                               
  zeroclaw agent -a max -m "用 comfyui 的 MCP 工具生成一张图,不要写本地工作流,不要用 stability 系列模型(接口 404)。步骤:1) partner_model_schema 查模型 grok 的参数;2) 
  partner_generate,model=grok,生成:一只橘猫坐在窗台上,方形构图,confirm_spend=true,out_path=/home/mil2er/.zeroclaw/agents/max/workspace/。我已同意这次扣费。完成后告诉我输出文件名"

```


