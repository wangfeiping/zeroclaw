#!/usr/bin/env bash
# Build the ComfyUI + ComfyUI-Manager + comfy-cli + comfy-mcp docker image.
#
#   ./build_docker_comfymcp_comfyui.sh
#   IMAGE_NAME=myrepo/comfyui IMAGE_TAG=dev ./build_docker_comfymcp_comfyui.sh --no-cache
set -euo pipefail

cd "$(dirname "$0")"

IMAGE_NAME="${IMAGE_NAME:-comfyui-comfymcp}"
IMAGE_TAG="${IMAGE_TAG:-$(sed -n 's/^__version__ = "\(.*\)"$/\1/p' comfyui_version.py)}"
TORCH_INDEX_URL="${TORCH_INDEX_URL:-https://download.pytorch.org/whl/cpu}"

docker build \
    --build-arg TORCH_INDEX_URL="${TORCH_INDEX_URL}" \
    -t "${IMAGE_NAME}:${IMAGE_TAG}" \
    -t "${IMAGE_NAME}:latest" \
    "$@" .

echo "Built ${IMAGE_NAME}:${IMAGE_TAG}"

# COMFY_API_KEY for the partner nodes goes in /opt/ai/comfy/.env
#
# docker run -d --name comfyui --restart unless-stopped \
#    --env-file /opt/ai/comfy/.env \
#    -p 127.0.0.1:8188:8188 \
#    -v /data/comfy/output:/app/ComfyUI/output \
#    -v /data/comfy/user:/app/ComfyUI/user \
#    comfyui-comfymcp
