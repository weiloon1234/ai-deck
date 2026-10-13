"""Opt-in, read-only GPUtw marker probe. No creation, writes or response dumps."""
import argparse
import getpass
import json
import re
import sys
import urllib.error
import urllib.request
import uuid

API_BASE = "https://api.gputw.ai/api"
MAX_RESPONSE_BYTES = 2 * 1024 * 1024


class ProbeError(Exception):
    pass


def unique_object(pairs):
    result = {}
    for name, value in pairs:
        if name in result:
            raise ProbeError("duplicate_json_field")
        result[name] = value
    return result


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


class GputwReadApi:
    def __init__(self, key):
        if (not isinstance(key, str) or len(key) > 512
                or not re.fullmatch(r"gputw_live_[A-Za-z0-9._~+/-]+=*", key)):
            raise ProbeError("invalid_key_format")
        self._key = key
        self._opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}), NoRedirect()
        )

    def _get(self, path):
        request = urllib.request.Request(
            API_BASE + path, method="GET",
            headers={"Authorization": "Bearer " + self._key,
                     "User-Agent": "AI-Deck-Recovery-Probe/1", "Accept": "application/json"},
        )
        try:
            with self._opener.open(request, timeout=20) as response:
                body = response.read(MAX_RESPONSE_BYTES + 1)
            if len(body) > MAX_RESPONSE_BYTES:
                raise ProbeError("response_too_large")
            envelope = json.loads(body, object_pairs_hook=unique_object)
        except urllib.error.HTTPError as error:
            code = error.code
            error.close()
            raise ProbeError("authentication_or_scope" if code in (401, 403)
                             else "rate_limited" if code == 429 else "http_error") from None
        except (urllib.error.URLError, OSError):
            raise ProbeError("network_error") from None
        except (ValueError, RecursionError):
            raise ProbeError("invalid_response") from None
        if not isinstance(envelope, dict) or envelope.get("success") is not True:
            raise ProbeError("invalid_response")
        return envelope.get("data")

    def list_instances(self):
        return self._get("/instances")

    def get_instance(self, instance_id):
        # Normalize before constructing the only resource-specific request path.
        return self._get("/instances/" + uuid_argument(instance_id))


def uuid_argument(value):
    try:
        parsed = uuid.UUID(value)
    except (ValueError, AttributeError):
        raise argparse.ArgumentTypeError("Use a hyphenated UUID.") from None
    if str(parsed) != value.lower():
        raise argparse.ArgumentTypeError("Use a hyphenated UUID.")
    return str(parsed)


def marker_argument(value):
    name, separator, marker = value.partition("=")
    if not separator or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]{0,127}", name):
        raise argparse.ArgumentTypeError("Use NAME=UUID for a non-secret environment marker.")
    uuid_argument(marker)
    return name, marker


def marker_observation(record, markers):
    """Inspect structure without returning server values, field paths or secrets."""
    matches = conflicts = visited = 0
    pending = [(record, 0)]
    while pending:
        value, depth = pending.pop()
        visited += 1
        if depth > 32 or visited > 20000:
            raise ProbeError("response_structure_too_large")
        values = None
        if isinstance(value, dict):
            if all(name in value for name in markers):
                values = value
            pending.extend((child, depth + 1) for child in value.values()
                           if isinstance(child, (dict, list)))
        elif isinstance(value, list):
            entries = {}
            for child in value:
                if isinstance(child, dict) and isinstance(child.get("name"), str):
                    name = child["name"]
                    if name in markers:
                        if name in entries:
                            raise ProbeError("duplicate_marker")
                        entries[name] = child.get("value")
                if isinstance(child, (dict, list)):
                    pending.append((child, depth + 1))
            if all(name in entries for name in markers):
                values = entries
        if values is not None:
            if all(values[name] == expected for name, expected in markers.items()):
                matches += 1
            else:
                conflicts += 1
    return {"matches": matches, "conflicts": conflicts}


def probe(api, instance_id, markers):
    records = api.list_instances()
    record = api.get_instance(instance_id)
    if (not isinstance(records, list) or not all(isinstance(item, dict) for item in records)
            or not isinstance(record, dict)):
        raise ProbeError("invalid_response")
    observations = [(item, marker_observation(item, markers)) for item in records]
    selected = [observation for item, observation in observations if item.get("id") == instance_id]
    matching_instances = sum(observation["matches"] > 0 for _, observation in observations)
    detail = marker_observation(record, markers)
    consistent = (len(selected) == 1 and matching_instances == 1
                  and selected[0]["matches"] > 0 and selected[0]["conflicts"] == 0
                  and record.get("id") == instance_id
                  and detail["matches"] > 0 and detail["conflicts"] == 0)
    return {"marker_readback": "observed" if consistent else "unconfirmed",
            "instance_listed_once": len(selected) == 1,
            "matching_instances": matching_instances,
            "list_marker_matches": selected[0]["matches"] if len(selected) == 1 else 0,
            "get_marker_matches": detail["matches"],
            "conflicting_markers": sum(item["conflicts"] for item in selected) + detail["conflicts"],
            "idempotency": "not_verified", "production_recovery": "not_enabled"}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instance-id", required=True, type=uuid_argument)
    parser.add_argument("--marker", required=True, action="append", type=marker_argument,
                        help="Repeat twice for the non-secret owner and deployment NAME=UUID markers.")
    args = parser.parse_args(argv)
    markers = dict(args.marker)
    if len(args.marker) != 2 or len(markers) != 2:
        parser.error("Supply two distinct non-secret marker names.")
    if not sys.stdin.isatty() or not sys.stderr.isatty():
        print(json.dumps({"error": "interactive_terminal_required"}))
        return 1
    try:
        key = getpass.getpass("GPUtw read-only API key (not saved): ").strip()
        result = probe(GputwReadApi(key), args.instance_id, markers)
    except ProbeError as error:
        print(json.dumps({"error": str(error)}))
        return 1
    except (KeyboardInterrupt, EOFError):
        print(json.dumps({"error": "cancelled"}))
        return 1
    print(json.dumps(result, indent=2))
    return 0 if result["marker_readback"] == "observed" else 2


if __name__ == "__main__":
    sys.exit(main())
