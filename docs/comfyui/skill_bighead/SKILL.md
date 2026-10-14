---
name: bighead
description: Put the face from a selfie into a famous painting as a big-head caricature, using ComfyUI and the grok-edit model, and send the image back.
version: 0.1.0
author: wangfeiping
tags: [slash, image, comfyui, grok]
slash_options:
  - name: painting
    description: Image URL of the painting (optional; or attach the images to a normal message).
    type: string
    required: false
  - name: scale
    description: How big the head should be (default 2x).
    type: string
    required: false
    choices: [{name: "1.5x", value: "1.5"}, {name: "2x", value: "2"}, {name: "3x", value: "3"}]
  - name: aspect_ratio
    description: Output aspect ratio (default 3:4).
    type: string
    required: false
    choices: [{name: "3:4", value: "3:4"}, {name: "2:3", value: "2:3"}, {name: "1:1", value: "1:1"}, {name: "4:3", value: "4:3"}, {name: "auto", value: "auto"}]
  - name: note
    description: Extra wishes, e.g. keep my glasses, smile.
    type: string
    required: false
---

# Famous painting big-head portrait

Take two images, a selfie and a famous painting, and generate one image: the painting with its subject's face replaced by the selfie's face, repainted in the painting's style, with the head enlarged like a bobblehead caricature.

Use this skill when the user asks for a 名画大头照 / big-head portrait / "put my face into this painting", or invokes `/bighead`.

## Inputs

Images reach you as `[IMAGE:<absolute path>]` markers in the message. You need exactly two inputs:

| index | what | source |
|---|---|---|
| 0 | the selfie (the face to use) | an attached image |
| 1 | the painting | an attached image, the `painting:` URL, or an http(s) image URL in the message |

- Order: the first attached image is the selfie and the second is the painting, unless the user says otherwise. If you can see the images and the order is clearly reversed (the first one is a painting), swap them.
- If the current message has fewer than two inputs, use the most recent images the same user sent earlier in this conversation.
- A slash command cannot carry attachments. If you still do not have two inputs, reply "请在同一条消息里 @我 并附上两张图：第一张自拍，第二张名画（名画也可以是图片链接）" and stop.
- Options: `scale` is 1.5, 2 or 3 (default 2). `aspect_ratio` defaults to `3:4`. `note` is optional extra wishes.

## Steps

1. Upload both inputs. Run with the `shell` tool from the workspace directory, selfie first:

   ```
   python3 skills/bighead/scripts/upload.py '<selfie path>' '<painting path or URL>'
   ```

   Quote each argument with single quotes. Read the output:
   - `IMAGE_URL_0=<url>` and `IMAGE_URL_1=<url>`: success. Use the URLs exactly as printed.
   - `BIGHEAD_ERROR=<reason>`: tell the user the reason in one short sentence and stop. For `cannot read image`, ask for a JPG or PNG (iPhone HEIC photos are not supported).

   Never call `docker` yourself; the script is the only part of this skill that touches the container. Uploading spends no credits.
2. Create the output directory: `mkdir -p comfy_output`.
3. Build the prompt from this template, replacing `{scale}`:

   ```
   Turn <IMAGE_1> into a playful big-head caricature portrait. Replace the face of the main subject in <IMAGE_1> with the face of the person in <IMAGE_0>, keeping that person's facial features, face shape, skin tone, hairstyle and expression recognizable. Repaint the new face in exactly the same artistic medium, brushwork, lighting, texture and color palette as <IMAGE_1> so it looks like part of the original painting. Enlarge the head to about {scale} times its natural size relative to the body, like a bobblehead, while keeping the original pose, clothing, background and composition of <IMAGE_1>. One single image, no text, no border.
   ```

   If the user gave a `note:` or other wishes, append one sentence: `Additional request: <note>` (keep the user's wording).
4. Call `comfyui__partner_generate`. The model's inputs go INSIDE `params`:

   ```json
   {
     "model": "grok-edit",
     "params": {
       "prompt": "<prompt>",
       "images": [
         {"url": "<IMAGE_URL_0>", "type": "image_url"},
         {"url": "<IMAGE_URL_1>", "type": "image_url"}
       ],
       "aspect_ratio": "3:4",
       "resolution": "1k"
     },
     "confirm_spend": true,
     "out_path": "<absolute workspace path>/comfy_output/",
     "timeout_seconds": 300
   }
   ```

   - `images` is a JSON array of objects, not a string. Do not use the single `image` field.
   - Keep `resolution` at `1k` unless the user asks for higher quality (`2k` costs more).
   - `out_path` is the absolute path of YOUR OWN workspace plus `/comfy_output/`, with the trailing slash.
5. The downloaded file is the first entry of the `saved` array in the result (some versions name it `saved_paths`). Do not use the `url` inside `result`: it is a temporary signed link. If neither array has an entry, report that the download failed and stop.
6. Reply with one short line plus the marker, using the exact absolute path:

   ```
   名画大头照已生成：
   [IMAGE:<absolute path>]
   ```

## Spending

Each `comfyui__partner_generate` call spends the owner's Comfy credits (about $0.02 to $0.05 per image; not yet measured for `grok-edit`).

- A big-head request or a `/bighead` invocation is consent for ONE generation. Set `confirm_spend` to true for that call only.
- One image per request. Do not generate variations or try other models unless asked.
- At most one retry after an API error, with the same parameters. If the error says the content was refused or moderated, do not retry; tell the user in one short sentence.
- Never switch to another partner model (`nano-banana`, `flux-kontext-*`, ...) without asking the user first; they cost more.

## Rules

- The second image must be an artwork (painting, drawing, sculpture photo). If it is a photo of a real person, decline: this skill does not swap faces between real photos.
- Decline requests meant to mock, sexualize, or impersonate someone, or to put a face the user says belongs to someone who has not agreed.
- The tools `comfyui__*` are already in your tool list; call them directly. Do not call the ComfyUI MCP server through `docker exec`, and do not write or run local workflows.
- Do not modify, move, or delete files outside `comfy_output/`.
- Do not call `comfyui__install_node`, `comfyui__download_model`, `comfyui__update_comfyui`, `comfyui__stop_comfyui`, `comfyui__restart_comfyui`, or `comfyui__auth_login`.
