"""No model downloads, GPU, cloud accounts, or paid inference. Standard library only."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import queue
import sys
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

SOURCE = Path(__file__).parents[2] / "containers/vllm-runtime/launcher.py"
CONFIG = {"repository": "fixture/model", "revision": "a" * 40, "statusPort": 8001, "inferencePort": 8000,
          "downloadTimeoutSeconds": 60, "loadTimeoutSeconds": 60, "minCacheGb": 1, "arguments": []}
with patch.dict(os.environ, {"RUNPOD_DECK_RUNTIME": json.dumps(CONFIG), "VLLM_API_KEY": "fixture-endpoint-token"}):
    spec = importlib.util.spec_from_file_location("launcher", SOURCE)
    launcher = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(launcher)

class RuntimeTests(unittest.TestCase):
    def hub(self, failure=None):
        hub = types.ModuleType("huggingface_hub")
        hub.HfApi = Mock()
        hub.HfApi.return_value.model_info.return_value = types.SimpleNamespace(siblings=[types.SimpleNamespace(rfilename="model.safetensors")])
        hub.get_hf_file_metadata = Mock(side_effect=failure)
        hub.hf_hub_url = Mock(return_value="https://fixture.invalid/weight")
        hub.snapshot_download = Mock()
        return hub

    def download(self, hub, space=True):
        results = queue.Queue()
        with patch.dict(sys.modules, {"huggingface_hub": hub}), patch.dict(os.environ, {"HF_TOKEN": "fixture-hf-token", "HF_HUB_CACHE": "/fixture/cache"}), patch.object(launcher, "cache_has_space", side_effect=space if isinstance(space, Exception) else None, return_value=space), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            launcher.download(results)
        return results.get_nowait()

    def test_protected_weight_is_checked_before_download_and_revision_is_pinned(self):
        hub = self.hub()
        self.assertEqual(self.download(hub), "downloaded")
        hub.get_hf_file_metadata.assert_called_once()
        call = hub.snapshot_download.call_args.kwargs
        self.assertEqual(call["revision"], CONFIG["revision"])
        self.assertEqual(call["token"], "fixture-hf-token")
        self.assertEqual(call["cache_dir"], "/fixture/cache")
        self.assertNotIn("local_dir", call)

    def test_access_failures_do_not_download_and_only_return_sanitized_codes(self):
        for status, code in [(401, "token_permissions"), (403, "model_access_denied"), (404, "model_not_found"), (500, "download_failed")]:
            error = Exception("sensitive-token-bearing-upstream-url")
            error.response = types.SimpleNamespace(status_code=status)
            hub = self.hub(error)
            self.assertEqual(self.download(hub), code)
            hub.snapshot_download.assert_not_called()

    def test_missing_weights_fail_before_any_download(self):
        hub = self.hub(); hub.HfApi.return_value.model_info.return_value.siblings = []
        self.assertEqual(self.download(hub), "weights_missing")
        hub.snapshot_download.assert_not_called()

    def test_insufficient_or_unreadable_cache_prevents_download(self):
        for space, expected in [(False, "insufficient_cache_space"), (OSError("fixture path"), "cache_space_unavailable")]:
            hub = self.hub()
            self.assertEqual(self.download(hub, space), expected)
            hub.snapshot_download.assert_not_called()

    def test_space_check_credits_only_complete_current_revision_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            snapshot = root / "models--fixture--model" / "snapshots" / CONFIG["revision"]
            snapshot.mkdir(parents=True)
            blob = root / "blob"; blob.write_bytes(b"x" * 200)
            (snapshot / "model.safetensors").symlink_to(blob)
            (snapshot / "duplicate.safetensors").symlink_to(blob)
            (snapshot / "incomplete.safetensors").symlink_to(root / "missing")
            other = snapshot.parent / "other-revision"; other.mkdir()
            (other / "weights.bin").write_bytes(b"x" * 500)
            outside = root.parent / (root.name + "-outside")
            try:
                outside.write_bytes(b"x" * 500)
                (snapshot / "outside.bin").symlink_to(outside)
                with patch.dict(os.environ, {"HF_HUB_CACHE": directory}), patch.object(launcher.shutil, "disk_usage") as disk:
                    disk.return_value.free = 1_000_000_000 - 201
                    self.assertFalse(launcher.cache_has_space())
                    disk.return_value.free += 1
                    self.assertTrue(launcher.cache_has_space())
            finally:
                outside.unlink(missing_ok=True)

    def test_runtime_logs_expose_categories_without_raw_output(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            launcher.monitor_logs(io.StringIO("sensitive unrelated output\nCUDA out of memory: fixture-secret\n"))
        self.assertNotIn("fixture-secret", output.getvalue())
        self.assertNotIn("sensitive", output.getvalue())
        self.assertEqual(json.loads(output.getvalue())["code"], "gpu_out_of_memory")

    def test_status_authentication_is_required_before_status_or_path_disclosure(self):
        for path, auth, expected in [("/status", "", 401), ("/status", "Bearer bad", 401), ("/elsewhere", "Bearer fixture-endpoint-token", 404), ("/status", "Bearer fixture-endpoint-token", 200)]:
            handler = object.__new__(launcher.StatusHandler)
            handler.path = path; handler.headers = {"Authorization": auth}; handler.wfile = io.BytesIO()
            handler.send_response = Mock(); handler.end_headers = Mock(); handler.send_header = Mock()
            handler.do_GET(); handler.send_response.assert_called_once_with(expected)
            self.assertNotIn(b"fixture-endpoint-token", handler.wfile.getvalue())
            if expected != 200: self.assertEqual(handler.wfile.getvalue(), b"")

    def test_non_ready_success_response_times_out_and_cleans_up_runtime_process_group(self):
        for refuses_termination in [False, True]:
            with self.subTest(refuses_termination=refuses_termination):
                context = Mock()
                context.Process.return_value.is_alive.return_value = False
                context.Queue.return_value.get.return_value = "downloaded"
                process = Mock(pid=1234, stdout=io.StringIO())
                process.poll.return_value = None
                if refuses_termination:
                    process.wait.side_effect = [launcher.subprocess.TimeoutExpired("fixture-runtime", 15), None]
                response = Mock(status=204)
                response.__enter__ = Mock(return_value=response)
                response.__exit__ = Mock(return_value=False)
                stop = Mock()
                stop.wait.side_effect = [False, True, True]
                stop.is_set.return_value = False
                output = io.StringIO()
                with contextlib.ExitStack() as stack:
                    stack.enter_context(contextlib.redirect_stdout(output))
                    stack.enter_context(patch.object(launcher, "STATE", {"stage": "containerStarting", "code": "starting"}))
                    stack.enter_context(patch.object(launcher, "STOP", stop))
                    server = stack.enter_context(patch.object(launcher.http.server, "ThreadingHTTPServer"))
                    stack.enter_context(patch.object(launcher.threading, "Thread"))
                    stack.enter_context(patch.object(launcher.signal, "signal"))
                    stack.enter_context(patch.object(launcher.multiprocessing, "get_context", return_value=context))
                    stack.enter_context(patch.object(launcher.subprocess, "Popen", return_value=process))
                    stack.enter_context(patch.object(launcher.urllib.request, "urlopen", return_value=response))
                    stack.enter_context(patch.object(launcher.time, "monotonic", side_effect=[0, 0, CONFIG["loadTimeoutSeconds"] + 1]))
                    kill_group = stack.enter_context(patch.object(launcher.os, "killpg"))
                    launcher.main()
                    self.assertEqual(launcher.STATE, {"stage": "failed", "code": "load_timeout"})
                kill_group.assert_any_call(process.pid, launcher.signal.SIGTERM)
                if refuses_termination:
                    kill_group.assert_any_call(process.pid, launcher.signal.SIGKILL)
                    self.assertEqual(kill_group.call_count, 2)
                    self.assertEqual(process.wait.call_count, 2)
                    process.wait.assert_called_with()
                else:
                    kill_group.assert_called_once_with(process.pid, launcher.signal.SIGTERM)
                    process.wait.assert_called_once_with(timeout=15)
                server.return_value.shutdown.assert_called_once_with()
                statuses = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(statuses[-1], {"stage": "failed", "code": "load_timeout"})
                self.assertNotIn("fixture-endpoint-token", output.getvalue())

if __name__ == "__main__":
    unittest.main()
