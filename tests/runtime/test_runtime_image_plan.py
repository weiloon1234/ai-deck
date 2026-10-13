"""Offline wrapper planning; no Docker, image pulls, keys or cloud calls."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).parents[2] / "scripts/prepare_runtime_image.py"
spec = importlib.util.spec_from_file_location("runtime_image_plan", SOURCE)
planner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(planner)


class RuntimeImagePlanTests(unittest.TestCase):
    def fixture(self, root):
        (root / "model-catalog").mkdir()
        catalog = {"runtimes": [{"id": "fixture", "image": "fixture/runtime@sha256:" + "a" * 64}]}
        (root / "model-catalog/catalog.json").write_text(json.dumps(catalog))
        context = root / "containers/vllm-runtime"
        context.mkdir(parents=True)
        for name in ["Dockerfile", ".dockerignore", "launcher.py"]:
            (context / name).write_text("fixture " + name)
        return catalog

    def test_plan_uses_selected_pinned_base_and_is_explicitly_unbuilt(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog = self.fixture(root)
            plan = planner.build_plan(root, "fixture", "registry.example.com/team/runtime")
            self.assertEqual(plan["baseImage"], catalog["runtimes"][0]["image"])
            self.assertIn("BASE_IMAGE=" + plan["baseImage"], plan["buildArgv"])
            self.assertEqual(plan["buildArgv"][-1], str(root / "containers/vllm-runtime"))
            self.assertEqual(plan["platform"], "linux/amd64")
            self.assertIsNone(plan["deploymentImage"])
            for flag in ["imageBuilt", "imagePublished", "providerEnabled"]:
                self.assertFalse(plan[flag])

    def test_source_or_base_changes_invalidate_recipe_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog = self.fixture(root)
            original = planner.build_plan(root, "fixture", "registry.example.com/team/runtime")
            self.assertEqual(original, planner.build_plan(root, "fixture", "registry.example.com/team/runtime"))
            previous = original
            for name in ["Dockerfile", ".dockerignore", "launcher.py"]:
                file = root / "containers/vllm-runtime" / name
                file.write_text(file.read_text() + " changed")
                changed = planner.build_plan(root, "fixture", "registry.example.com/team/runtime")
                self.assertNotEqual(changed["localImageTag"], previous["localImageTag"])
                previous = changed
            catalog["runtimes"][0]["image"] = "fixture/runtime@sha256:" + "b" * 64
            (root / "model-catalog/catalog.json").write_text(json.dumps(catalog))
            changed_base = planner.build_plan(root, "fixture", "registry.example.com/team/runtime")
            self.assertNotEqual(changed_base["recipeFingerprint"], changed["recipeFingerprint"])

    def test_unpinned_duplicate_unknown_runtime_and_unsafe_repository_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog = self.fixture(root)
            for repository in ["runtime", "https://registry.example.com/team/runtime", "registry.example.com/user:secret@runtime",
                               "registry.example.com/team/runtime:latest", "registry.example.com/team/$(whoami)"]:
                with self.assertRaises(ValueError):
                    planner.build_plan(root, "fixture", repository)
            with self.assertRaises(ValueError):
                planner.build_plan(root, "missing", "registry.example.com/team/runtime")
            catalog["runtimes"].append(catalog["runtimes"][0])
            (root / "model-catalog/catalog.json").write_text(json.dumps(catalog))
            with self.assertRaises(ValueError):
                planner.build_plan(root, "fixture", "registry.example.com/team/runtime")
            catalog["runtimes"] = [{"id": "fixture", "image": "fixture/runtime:latest"}]
            (root / "model-catalog/catalog.json").write_text(json.dumps(catalog))
            with self.assertRaises(ValueError):
                planner.build_plan(root, "fixture", "registry.example.com/team/runtime")

    def test_cli_prints_plan_and_sanitizes_invalid_input(self):
        output, error = io.StringIO(), io.StringIO()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            with patch.object(planner, "ROOT", root), contextlib.redirect_stdout(output):
                self.assertEqual(planner.main(["--runtime-id", "fixture", "--image-repository",
                                               "registry.example.com/team/runtime"]), 0)
            self.assertFalse(json.loads(output.getvalue())["providerEnabled"])
            with patch.object(planner, "ROOT", root), contextlib.redirect_stderr(error):
                self.assertEqual(planner.main(["--runtime-id", "fixture", "--image-repository",
                                               "registry.example.com/user:fixture-secret@runtime"]), 1)
            self.assertNotIn("fixture-secret", error.getvalue())

    def test_source_repository_binds_registry_link_and_recipe_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            plain = planner.build_plan(root, "fixture", "registry.example.com/team/runtime")
            source = "https://github.com/fixture-owner/runtime"
            linked = planner.build_plan(root, "fixture", "registry.example.com/team/runtime", source)
            self.assertIn("org.opencontainers.image.source=" + source, linked["buildArgv"])
            self.assertIn("io.ai-deck.recipe.sha256=" + linked["recipeFingerprint"], linked["buildArgv"])
            self.assertNotEqual(plain["localImageTag"], linked["localImageTag"])
            for invalid in ["http://github.com/owner/runtime", "https://user:fixture-secret@github.com/owner/runtime",
                            "https://github.com.evil/owner/runtime", "https://github.com/owner/runtime?token=fixture-secret"]:
                with self.assertRaises(ValueError):
                    planner.build_plan(root, "fixture", "registry.example.com/team/runtime", invalid)


if __name__ == "__main__":
    unittest.main()
