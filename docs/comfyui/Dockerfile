FROM python:3.12-slim

# CPU only: the container runs API nodes, no local models. See README.md for the GPU index urls.
ARG TORCH_INDEX_URL=https://download.pytorch.org/whl/cpu

ENV PYTHONUNBUFFERED=1 \
    PIP_NO_CACHE_DIR=1 \
    DO_NOT_TRACK=1

# git is needed by ComfyUI-Manager to install custom nodes, libgl1/libglib2.0-0 by opencv based custom nodes.
RUN apt-get update \
    && apt-get install -y --no-install-recommends git libgl1 libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app/ComfyUI

COPY requirements.txt manager_requirements.txt ./
RUN pip install torch torchvision torchaudio --extra-index-url ${TORCH_INDEX_URL} \
    && pip install -r requirements.txt -r manager_requirements.txt "comfy-cli>=1.14" comfy-mcp

COPY . .

# comfy-mcp shells out to comfy-cli, which needs to know where the workspace is.
RUN comfy --skip-prompt set-default /app/ComfyUI

VOLUME ["/app/ComfyUI/models", "/app/ComfyUI/custom_nodes", "/app/ComfyUI/input", "/app/ComfyUI/output", "/app/ComfyUI/user"]

EXPOSE 8188

CMD ["python", "main.py", "--listen", "0.0.0.0", "--port", "8188", "--enable-manager", "--cpu"]
