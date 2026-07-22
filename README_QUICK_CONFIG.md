# 配置

zeroclaw onboard          # 交互式向导，~9 个问题，2 分钟内完成，选择kimi-code，配置相关api-key 和model-id

zeroclaw service install  # systemd / OpenRC
zeroclaw service start

# 或

systemctl --user status zeroclaw

# 需要特别关注的配置项

vi ~/.zeroclaw/config.toml

#temperature = 0.7 默认值需要修改
temperature = 1
api_key = "enc2:***"
model = "kimi-code-2.5"

# 启动命令行对话

zeroclaw agent

