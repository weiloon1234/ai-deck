# Reusable vLLM runtime

The pinned upstream image is defined once in `model-catalog/catalog.json`. Both candidate models reference it. A separate Docker image is not required: the native provisioner passes `launcher.py` as one structured Python argument in `dockerStartCmd`, with `dockerEntrypoint` set to `python3 -u -c`. No shell interpolates model arguments or credentials.

The supervisor exposes a small authenticated status service, checks access to a weight at the immutable revision, checks the catalog’s `minCacheGb` requirement against free space plus complete files already cached for that revision, downloads missing files to the selected revision cache, and starts vLLM directly. Inference streams through vLLM's own authenticated API; there is no response translation layer.

Download and load timeouts come from the runtime profile. Only fixed status/error categories reach stdout or the status endpoint. Hugging Face errors and raw vLLM logs are not forwarded. The controller verifies rejected bad authentication, model identity and real inference before Ready, and separately controls Enable.

`HF_HOME` stays in temporary container storage; weights and compilation caches use explicit distinct paths. Compilation caches are keyed by the resolved profile fingerprint. No login is performed and no management key is passed into the Pod. Protected profiles reference a Runpod Secret using the provider's documented template syntax.

Stopping the supervisor/container is not cloud cleanup and does not prove billing stopped. Failed startup leaves authenticated diagnostic status available until the controller verifies Pod deletion. Retained network volumes are never deleted by this runtime or normal Finish.

Run the offline tests with:

```sh
python3 -m unittest discover -s tests/runtime -v
```

These tests mock downloads and runtime output. Docker startup, GPU kernels, model loading, both inference protocols, cache reuse and real throughput remain owner-led cloud validation gates. See `docs/model-compatibility.md`.
