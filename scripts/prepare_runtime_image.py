"""Print a catalog-bound wrapper build plan; never invoke Docker or a registry."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]


def build_plan(root, runtime_id, image_repository, source_repository=None):
    # Require an explicit registry and repository, without credentials/tags.
    component = r"[a-z0-9]+(?:[._-][a-z0-9]+)*"
    if not re.fullmatch(component + r"\.(?:" + component + r")/(?:" + component + r"/)*" + component,
                        image_repository):
        raise ValueError("Use a lowercase registry/repository without a tag or credentials.")
    if source_repository is not None and not re.fullmatch(
            r"https://github.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", source_repository):
        raise ValueError("Use a GitHub repository URL without credentials, query or fragment.")
    catalog = json.loads((root / "model-catalog/catalog.json").read_text())
    matches = [runtime for runtime in catalog["runtimes"] if runtime["id"] == runtime_id]
    if len(matches) != 1:
        raise ValueError("Select one existing catalog runtime ID.")
    image = matches[0]["image"]
    if not re.fullmatch(r"[a-z0-9./_-]+@sha256:[a-f0-9]{64}", image):
        raise ValueError("The catalog runtime must use a SHA-256 image digest.")
    context = root / "containers/vllm-runtime"
    sources = {name: hashlib.sha256((context / name).read_bytes()).hexdigest()
               for name in ("Dockerfile", ".dockerignore", "launcher.py")}
    identity = {"baseImage": image, "sourceSha256": sources, "platform": "linux/amd64",
                "sourceRepository": source_repository}
    fingerprint = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
    tagged_image = image_repository + ":ai-deck-" + fingerprint[:24]
    labels = ["--label", "io.ai-deck.recipe.sha256=" + fingerprint]
    if source_repository is not None:
        labels += ["--label", "org.opencontainers.image.source=" + source_repository]
    return {
        "runtimeId": runtime_id, **identity, "recipeFingerprint": fingerprint,
        "localImageTag": tagged_image,
        "buildArgv": ["docker", "build", "--platform", identity["platform"],
                      "--build-arg", "BASE_IMAGE=" + image, "--tag", tagged_image,
                      *labels, str(context)],
        "imageBuilt": False, "imagePublished": False, "providerEnabled": False,
        "deploymentImage": None,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime-id", required=True)
    parser.add_argument("--image-repository", required=True,
                        help="Explicit registry/repository for the future wrapper, without a tag")
    parser.add_argument("--source-repository", help="Optional GitHub URL to link the image package")
    args = parser.parse_args(argv)
    try:
        plan = build_plan(ROOT, args.runtime_id, args.image_repository, args.source_repository)
    except (ValueError, KeyError, TypeError, OSError):
        print("Select a valid catalog runtime and registry/repository; ensure recipe files exist.",
              file=sys.stderr)
        return 1
    print(json.dumps(plan, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
