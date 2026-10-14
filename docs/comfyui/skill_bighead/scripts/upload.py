"""Upload images to Comfy storage and print signed URLs for grok-edit.

    python3 skills/bighead/scripts/upload.py <selfie> <painting>

Each argument is a local image path or an http(s) URL.

Output, one line per input in argument order:
    IMAGE_URL_0=<url>
    IMAGE_URL_1=<url>
or a single line on failure:
    BIGHEAD_ERROR=<reason>

An http(s) URL is first downloaded on the host (max 100 MB). Each image is
then copied into the ComfyUI container, converted to JPEG with the longest side
at most BIGHEAD_MAX_SIDE pixels, and uploaded with `comfy generate upload`.
Uploading spends no credits. The URLs expire after 24 hours.

Standard library only; docker must be usable by the current user.
"""

from __future__ import annotations

import json
import os
import secrets
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

CONTAINER = os.environ.get("COMFY_CONTAINER", "comfyui")
MAX_SIDE = int(os.environ.get("BIGHEAD_MAX_SIDE", "2048"))
MAX_DOWNLOAD_BYTES = 100 * 1024 * 1024
MAX_INPUTS = 5
USER_AGENT = "zeroclaw-bighead/0.1 (image download)"

# Runs inside the container (which has Pillow): <src> <dst.jpg> <max side>.
CONVERT_PY = """
import sys
from PIL import Image, ImageOps
src, dst, max_side = sys.argv[1], sys.argv[2], int(sys.argv[3])
try:
    im = Image.open(src)
    im = ImageOps.exif_transpose(im)
except Exception as e:
    sys.exit("cannot read image (%s)" % type(e).__name__)
im = im.convert("RGB")
im.thumbnail((max_side, max_side))
im.save(dst, "JPEG", quality=90)
"""


class BigheadError(Exception):
    pass


def run(args: list[str], timeout: float) -> str:
    try:
        proc = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    except FileNotFoundError:
        raise BigheadError(f"{args[0]} not found")
    except subprocess.TimeoutExpired:
        raise BigheadError(f"{' '.join(args[:3])} timed out")
    if proc.returncode != 0:
        lines = (proc.stderr or proc.stdout).strip().splitlines()
        raise BigheadError(lines[-1] if lines else f"exit code {proc.returncode}")
    return proc.stdout


def download(url: str, dest: Path) -> None:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(req, timeout=60) as resp, dest.open("wb") as f:
            total = 0
            while chunk := resp.read(1 << 20):
                total += len(chunk)
                if total > MAX_DOWNLOAD_BYTES:
                    raise BigheadError("download larger than 100 MB")
                f.write(chunk)
    except BigheadError:
        raise
    except Exception as e:
        raise BigheadError(f"download failed ({e})")


def parse_url(output: str) -> str:
    start = output.find("{")
    try:
        obj = json.loads(output[start:]) if start >= 0 else {}
    except ValueError:
        obj = {}
    data = obj.get("data") if isinstance(obj.get("data"), dict) else obj
    return data.get("url") or ""


def upload_one(index: int, src: str, workdir: Path, tag: str) -> str:
    if src.startswith(("http://", "https://")):
        local = workdir / str(index)
        download(src, local)
    else:
        local = Path(src).expanduser()
        if not local.is_file():
            raise BigheadError(f"file not found: {src}")

    raw = f"/tmp/{tag}-{index}-src"
    jpg = f"/tmp/{tag}-{index}.jpg"
    run(["docker", "cp", str(local), f"{CONTAINER}:{raw}"], timeout=120)
    run(["docker", "exec", CONTAINER, "python", "-c", CONVERT_PY, raw, jpg, str(MAX_SIDE)], timeout=120)
    out = run(["docker", "exec", CONTAINER, "comfy", "generate", "upload", jpg, "--json"], timeout=180)
    url = parse_url(out)
    if not url:
        raise BigheadError("upload returned no url")
    return url


def main(argv: list[str]) -> int:
    if not argv:
        print("BIGHEAD_ERROR=usage: upload.py <image> [<image> ...]")
        return 1
    if len(argv) > MAX_INPUTS:
        print(f"BIGHEAD_ERROR=too many images (max {MAX_INPUTS})")
        return 1

    tag = f"bighead-{secrets.token_hex(4)}"
    urls: list[str] = []
    try:
        state = run(["docker", "inspect", "-f", "{{.State.Running}}", CONTAINER], timeout=30)
        if state.strip() != "true":
            raise BigheadError(f"container {CONTAINER} is not running")
        with tempfile.TemporaryDirectory(prefix="bighead-") as tmp:
            for i, src in enumerate(argv):
                try:
                    urls.append(upload_one(i, src, Path(tmp), tag))
                except BigheadError as e:
                    raise BigheadError(f"image {i}: {e}")
    except BigheadError as e:
        print(f"BIGHEAD_ERROR={e}")
        return 1
    finally:
        subprocess.run(
            ["docker", "exec", CONTAINER, "sh", "-c", f"rm -f /tmp/{tag}-*"],
            capture_output=True,
            timeout=30,
        )

    for i, url in enumerate(urls):
        print(f"IMAGE_URL_{i}={url}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
