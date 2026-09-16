# Discord

```text
我想配置dicord作为zeroclaw的channel                                                                                                                                                         

```

  ● 这个之前详细聊过，这里给你精简版供直接照抄使用（如果想看完整细节可以翻上面）。

  1. Discord 开发者后台准备

  去 https://discord.com/developers/applications 建一个 Application + Bot，拿到 Bot Token，并在 Bot 设置里打开 MESSAGE CONTENT INTENT（必须，否则收不到消息内容）。用 OAuth2 URL Generator
  生成邀请链接（勾 bot scope）把机器人邀请进服务器。

  2. 配置文件 ~/.zeroclaw/config.toml

  [channels]
  discord = true

  [channels_config.discord]
  enabled = true
  bot_token = "你的Bot Token"
  allowed_users = ["你的Discord用户ID"]   # 留空=拒绝所有人，务必填
  mention_only = true                     # 只响应 @机器人 的消息

  3. 或者用 CLI 直接加

  zeroclaw channel add discord '{"bot_token":"你的token","name":"my-discord"}'

  4. 生效 + 验证

  systemctl --user restart zeroclaw   # 或对应的重启方式
  zeroclaw channels doctor

  最容易踩的坑：allowed_users 忘记加自己ID（默认拒绝所有人）、Developer Portal 里 MESSAGE CONTENT INTENT 没勾。

