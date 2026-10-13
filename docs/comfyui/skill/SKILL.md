---
name: comfyui
description: Generate an image or a short video from a text prompt with ComfyUI partner models and send the file back.
version: 0.1.0
author: wangfeiping
tags: [slash, image, video, comfyui]
slash_options:
  - name: prompt
    description: What to generate.
    type: string
    required: true
  - name: type
    description: Image or video (default image).
    type: string
    required: false
    choices: [{name: Image, value: "image"}, {name: Video, value: "video"}]
  - name: duration
    description: Video length in seconds, 1-15 (default 5). Ignored for images.
    type: string
    required: false
  - name: aspect_ratio
    description: Aspect ratio (default 1:1 for images, 16:9 for videos).
    type: string
    required: false
    choices: [{name: "1:1", value: "1:1"}, {name: "16:9", value: "16:9"}, {name: "9:16", value: "9:16"}, {name: "4:3", value: "4:3"}, {name: "3:4", value: "3:4"}]
---

# ComfyUI image and video generation

Generate an image or a short video with the ComfyUI MCP server and return the file as an attachment.

Use this skill when the user asks to draw / generate / create an image, picture, or video, or invokes `/comfyui`. The request fields are `prompt:`, and optionally `type:`, `duration:`, `aspect_ratio:`. In a plain message, infer them from the text; default to an image.

## Tools

The tools come from the MCP server `comfyui` and are named `comfyui__<tool>`. They are already in your tool list. Call them directly.

- Do NOT check `mcp_resources` to decide whether ComfyUI is available: this server exposes tools only, so the resource list is always empty.
- Do NOT call the server through `shell` / `docker exec`.
- If `comfyui__partner_generate` is truly absent from your tool list, tell the user "ComfyUI MCP 未挂载到当前 agent" and stop.

## Models

The ComfyUI container is CPU-only and has no local models. Only hosted partner models work.

| type | model | notes |
|---|---|---|
| image | `grok` | default |
| video | `grok-video` | default, 1-15 seconds |

Fallbacks when the default returns an API error: image `recraft`, then `flux-2`. Never use `stability-*` models (the API returns 404). Never write or run a local workflow (`run_workflow`, `generate_image`, `validate_workflow`, checkpoints).

For any model other than the two defaults, call `comfyui__partner_model_schema` with `{"model": "<model>"}` first and use only the parameters it lists.

## Steps

1. Create the output directory with the `shell` tool from the workspace: `mkdir -p comfy_output`.
2. Call `comfyui__partner_generate`. The model's own inputs go INSIDE `params`, never at the top level.

   Image:

   ```json
   {
     "model": "grok",
     "params": {"prompt": "<prompt>", "aspect_ratio": "1:1"},
     "confirm_spend": true,
     "out_path": "<absolute workspace path>/comfy_output/",
     "timeout_seconds": 300
   }
   ```

   Video:

   ```json
   {
     "model": "grok-video",
     "params": {"prompt": "<prompt>", "duration": 5, "aspect_ratio": "16:9", "resolution": "480p"},
     "confirm_spend": true,
     "out_path": "<absolute workspace path>/comfy_output/",
     "timeout_seconds": 600
   }
   ```

   - `out_path` must be the absolute path of YOUR OWN workspace plus `/comfy_output/`, with the trailing slash. Never use another agent's directory.
   - `duration` is an integer. If the user gave none, use 5.
   - Keep `resolution` at `480p` unless the user asks for higher quality; higher resolutions cost more.
3. The tool result is JSON. The downloaded file is the first entry of its `saved` array:

   ```json
   {"result": {"...": "..."}, "saved": ["<absolute workspace path>/comfy_output/1ce063d1_0.jpg"]}
   ```

   Use `saved[0]` exactly as returned. Do not use the `url` inside `result`: it is a temporary signed link, not the attachment. If `saved` is empty, report that the download failed and stop.
4. Reply with one short line plus the marker, using the exact absolute path:

   ```
   已生成图片：
   [IMAGE:<absolute path>]
   ```

   For a video use `[VIDEO:<absolute path>]`.

## Spending

Every `comfyui__partner_generate` call spends the owner's Comfy credits (measured: a `grok` image about $0.02, a 5-second 480p `grok-video` about $0.25).

- An explicit generation request or a `/comfyui` invocation is consent for ONE call. Set `confirm_spend` to true for that call only.
- One generation per request. Do not generate variations, upscale, or retry with a more expensive model unless asked.
- At most one retry after a failure, and only with a fallback model from the table above. Then report the error in one short sentence and stop.
- Video longer than 15 seconds, or a request for more than one file: ask the user to confirm first.

## Rules

- Pass the user's prompt as given. Do not translate, expand, or embellish it unless the user asks you to improve it.
- A video takes from 30 seconds to several minutes. Do not start a second call while one is running.
- Do not modify, move, or delete files outside `comfy_output/`.
- Do not call `comfyui__install_node`, `comfyui__download_model`, `comfyui__update_comfyui`, `comfyui__stop_comfyui`, `comfyui__restart_comfyui`, or `comfyui__auth_login`.
