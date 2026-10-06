"""Reusable supervisor injected as structured Python entrypoint arguments.

No shell interpolation, management key, or protocol translation. The status port
is authenticated; inference streams directly through vLLM on its own port.
"""
import hmac
import http.server
import json
import multiprocessing
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import threading
import time
import urllib.request

CONFIG = json.loads(os.environ["RUNPOD_DECK_RUNTIME"])
TOKEN = os.environ["VLLM_API_KEY"]
STATE = {"stage": "containerStarting", "code": "starting"}
LOCK = threading.Lock()
STOP = threading.Event()


def report(stage, code):
    with LOCK:
        STATE.update(stage=stage, code=code)
    print(json.dumps({"stage": stage, "code": code}), flush=True)


class StatusHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if not hmac.compare_digest(self.headers.get("Authorization", ""), "Bearer " + TOKEN):
            self.send_response(401)
            self.end_headers()
            return
        if self.path != "/status":
            self.send_response(404)
            self.end_headers()
            return
        with LOCK:
            payload = json.dumps(STATE).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_):
        pass


def cache_has_space():
    cache = Path(os.environ["HF_HUB_CACHE"])
    cache.mkdir(parents=True, exist_ok=True)
    root = cache.resolve()
    snapshot = cache / ("models--" + CONFIG["repository"].replace("/", "--")) / "snapshots" / CONFIG["revision"]
    # Credit only complete files for this immutable revision, not other models
    # or unfinished blob downloads. Count shared blobs once, on this cache mount.
    reusable = set()
    for entry in snapshot.rglob("*"):
        target = entry.resolve()
        if target.is_relative_to(root) and target.is_file():
            reusable.add(target)
    cached_bytes = sum(p.stat().st_size for p in reusable)
    return shutil.disk_usage(cache).free + cached_bytes >= CONFIG["minCacheGb"] * 1_000_000_000


def download(result):
    # Library errors and download progress must not leak token-bearing URLs.
    with open(os.devnull, "w") as sink:
        sys.stdout = sink
        sys.stderr = sink
        try:
            from huggingface_hub import HfApi, get_hf_file_metadata, hf_hub_url, snapshot_download
            token = os.environ.get("HF_TOKEN")
            info = HfApi(token=token).model_info(CONFIG["repository"], revision=CONFIG["revision"])
            weights = [f.rfilename for f in info.siblings if f.rfilename.endswith((".safetensors", ".bin"))]
            if not weights:
                result.put("weights_missing")
                return
            # Check a protected weight, not just public repository metadata.
            get_hf_file_metadata(hf_hub_url(CONFIG["repository"], weights[0], revision=CONFIG["revision"]), token=token, timeout=30)
            try:
                enough_space = cache_has_space()
            except OSError:
                result.put("cache_space_unavailable")
                return
            if not enough_space:
                result.put("insufficient_cache_space")
                return
            snapshot_download(
                repo_id=CONFIG["repository"], revision=CONFIG["revision"], token=token,
                cache_dir=os.environ["HF_HUB_CACHE"],
                allow_patterns=["*.safetensors", "*.bin", "*.json", "*.model", "*.tiktoken", "*.txt", "*.jinja"],
            )
            result.put("downloaded")
        except Exception as error:
            response = getattr(error, "response", None)
            code = getattr(response, "status_code", None)
            result.put("token_permissions" if code == 401 else "model_access_denied" if code == 403 else "model_not_found" if code == 404 else "download_failed")


def monitor_logs(pipe):
    # Expose only recognized error categories, never arbitrary runtime output.
    for line in iter(pipe.readline, ""):
        lower = line.lower()
        if "out of memory" in lower or "cuda out of memory" in lower:
            report("failed", "gpu_out_of_memory")
        elif "unrecognized arguments" in lower or "invalid choice" in lower:
            report("failed", "runtime_configuration")


def main():
    server = http.server.ThreadingHTTPServer(("0.0.0.0", CONFIG["statusPort"]), StatusHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    signal.signal(signal.SIGTERM, lambda *_: STOP.set())
    signal.signal(signal.SIGINT, lambda *_: STOP.set())
    report("downloading", "checking_model_access")
    context = multiprocessing.get_context("fork")
    result = context.Queue()
    worker = context.Process(target=download, args=(result,))
    worker.start()
    deadline = time.monotonic() + CONFIG["downloadTimeoutSeconds"]
    while worker.is_alive() and not STOP.wait(0.2) and time.monotonic() < deadline:
        pass
    if worker.is_alive():
        worker.terminate()
        worker.join(10)
        if worker.is_alive():
            worker.kill()
        report("failed", "download_timeout")
    else:
        worker.join()
        try:
            code = result.get(timeout=2)
        except Exception:
            code = "download_failed"
        if code != "downloaded":
            report("failed", code)
        elif not STOP.is_set():
            report("loading", "loading_model")
            process = subprocess.Popen(
                [sys.executable, "-m", "vllm.entrypoints.openai.api_server", *CONFIG["arguments"]],
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, start_new_session=True,
            )
            threading.Thread(target=monitor_logs, args=(process.stdout,), daemon=True).start()
            deadline = time.monotonic() + CONFIG["loadTimeoutSeconds"]
            ready = False
            while process.poll() is None and not STOP.wait(1):
                if not ready:
                    try:
                        request = urllib.request.Request(
                            "http://127.0.0.1:%d/v1/models" % CONFIG["inferencePort"],
                            headers={"Authorization": "Bearer " + TOKEN},
                        )
                        with urllib.request.urlopen(request, timeout=2) as response:
                            ready = response.status == 200
                        if ready:
                            report("verifying", "awaiting_inference_verification")
                    except Exception:
                        if time.monotonic() >= deadline:
                            report("failed", "load_timeout")
                            break
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
            with LOCK:
                failed = STATE["stage"] == "failed"
            if not failed and not STOP.is_set():
                report("failed", "runtime_exited")
    # Keep authenticated diagnostics available for the controller to clean up.
    # Exiting a container is NOT Pod termination or a billing cap.
    while not STOP.wait(1):
        pass
    server.shutdown()


if __name__ == "__main__":
    main()
