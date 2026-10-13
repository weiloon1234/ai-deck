"""Offline tests with fake API replies and credentials; no GPUtw calls."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest.mock import Mock, patch
import urllib.error

SOURCE = Path(__file__).parents[2] / "scripts/probe_gputw_recovery.py"
spec = importlib.util.spec_from_file_location("gputw_probe", SOURCE)
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)
INSTANCE = "11111111-1111-4111-8111-111111111111"
MARKERS = {"FIXTURE_OWNER": "22222222-2222-4222-8222-222222222222",
           "FIXTURE_DEPLOYMENT": "33333333-3333-4333-8333-333333333333"}


class RecoveryProbeTests(unittest.TestCase):
    def api(self, records, detail):
        api = Mock()
        api.list_instances.return_value = records
        api.get_instance.return_value = detail
        return api

    def test_matching_markers_in_both_response_shapes_are_observations_only(self):
        uppercase = "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA"
        self.assertEqual(probe.marker_argument("FIXTURE_OWNER=" + uppercase), ("FIXTURE_OWNER", uppercase))
        listing = {"id": INSTANCE, "env": MARKERS}
        detail = {"id": INSTANCE, "customImage": {"env": [
            {"name": name, "value": value} for name, value in MARKERS.items()]}}
        api = self.api([listing], detail)
        result = probe.probe(api, INSTANCE, MARKERS)
        self.assertEqual(result["marker_readback"], "observed")
        self.assertEqual(result["idempotency"], "not_verified")
        self.assertEqual(result["production_recovery"], "not_enabled")
        api.get_instance.assert_called_once_with(INSTANCE)

    def test_missing_conflicting_duplicate_or_wrong_identity_stays_unconfirmed(self):
        valid = {"id": INSTANCE, "env": MARKERS}
        other = dict(MARKERS, FIXTURE_OWNER="wrong-owner")
        cases = [([], valid), ([valid], {"id": INSTANCE}),
                 ([valid, valid], valid), ([valid], dict(valid, id="another-instance")),
                 ([valid], {"id": INSTANCE, "env": MARKERS, "otherEnv": other}),
                 ([valid, dict(valid, id="another-instance")], valid),
                 ([valid], {"id": INSTANCE, "env": [{"FIXTURE_OWNER": MARKERS["FIXTURE_OWNER"]},
                                                   {"FIXTURE_DEPLOYMENT": MARKERS["FIXTURE_DEPLOYMENT"]}]})]
        for listing, detail in cases:
            with self.subTest(listing_count=len(listing)):
                self.assertEqual(probe.probe(self.api(listing, detail), INSTANCE, MARKERS)
                                 ["marker_readback"], "unconfirmed")

    def test_duplicate_named_markers_and_excessive_nesting_are_rejected(self):
        duplicate = [{"name": name, "value": value} for name, value in MARKERS.items()]
        duplicate.append(duplicate[0])
        nested = MARKERS
        for _ in range(34):
            nested = {"nested": nested}
        for value in [duplicate, nested]:
            with self.assertRaises(probe.ProbeError):
                probe.marker_observation(value, MARKERS)

    def test_transport_uses_only_fixed_host_gets_and_never_dumps_credentials(self):
        key = "gputw_live_fixture_secret"
        with patch.object(probe.urllib.request, "build_opener") as builder:
            api = probe.GputwReadApi(key)
        replies = [{"success": True, "data": []},
                   {"success": True, "data": {"id": INSTANCE}}]
        api._opener.open.side_effect = [io.BytesIO(json.dumps(item).encode()) for item in replies]
        self.assertEqual(api.list_instances(), [])
        self.assertEqual(api.get_instance(INSTANCE), {"id": INSTANCE})
        for call, suffix in zip(api._opener.open.call_args_list, ["/instances", "/instances/" + INSTANCE]):
            request = call.args[0]
            self.assertEqual(request.full_url, probe.API_BASE + suffix)
            self.assertEqual(request.get_method(), "GET")
            self.assertIsNone(request.data)
            self.assertEqual(request.get_header("Authorization"), "Bearer " + key)
        self.assertEqual(builder.call_args.args[0].proxies, {})
        self.assertIsNone(probe.NoRedirect().redirect_request(None, None, 302, "", {}, "https://fixture.invalid"))
        with self.assertRaises(Exception):
            api.get_instance("https://fixture.invalid/steal")
        self.assertEqual(api._opener.open.call_count, 2)

    def test_unexpected_oversized_and_http_errors_are_sanitized(self):
        with patch.object(probe.urllib.request, "build_opener"):
            api = probe.GputwReadApi("gputw_live_fixture_secret")
        for body in [b"private upstream text", b"x" * (probe.MAX_RESPONSE_BYTES + 1),
                     b'{"success":true,"data":[],"data":["private upstream text"]}',
                     json.dumps({"success": False, "error": "private upstream text"}).encode()]:
            api._opener.open.return_value = io.BytesIO(body)
            with self.assertRaises(probe.ProbeError) as error:
                api.list_instances()
            self.assertNotIn("private", str(error.exception))
        api._opener.open.side_effect = urllib.error.HTTPError(
            "https://fixture.invalid/private", 403, "private upstream text", {}, io.BytesIO(b"secret"))
        with self.assertRaisesRegex(probe.ProbeError, "^authentication_or_scope$"):
            api.list_instances()
        for key in ["gputw_live_", "gputw_live_fixture\r\nprivate", "gputw_live_fixture\x00private"]:
            with self.assertRaisesRegex(probe.ProbeError, "^invalid_key_format$"):
                probe.GputwReadApi(key)

    def test_cli_output_contains_only_sanitized_results_and_requires_a_terminal(self):
        args = ["--instance-id", INSTANCE]
        for name, value in MARKERS.items():
            args.extend(["--marker", name + "=" + value])
        output = io.StringIO()
        valid = {"id": INSTANCE, "env": dict(MARKERS, HF_TOKEN="fixture-private-token")}
        with patch.object(probe.sys.stdin, "isatty", return_value=True), \
                patch.object(probe.sys.stderr, "isatty", return_value=True), \
                patch.object(probe.getpass, "getpass", return_value="gputw_live_fixture_secret"), \
                patch.object(probe, "GputwReadApi", return_value=self.api([valid], valid)), \
                contextlib.redirect_stdout(output):
            self.assertEqual(probe.main(args), 0)
        for private in [INSTANCE, *MARKERS.values(), "fixture-private-token", "gputw_live_fixture_secret"]:
            self.assertNotIn(private, output.getvalue())
        with patch.object(probe.sys.stdin, "isatty", return_value=False), \
                patch.object(probe.getpass, "getpass") as prompt, \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(probe.main(args), 1)
            prompt.assert_not_called()


if __name__ == "__main__":
    unittest.main()
