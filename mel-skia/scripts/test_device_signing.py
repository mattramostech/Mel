"""Signing contract tests: synthetic profiles only, no keychain or device access."""

import copy
import contextlib
import datetime as dt
import hashlib
import importlib.util
import io
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("run_device", Path(__file__).with_name("run-device.py"))
device_runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(device_runner)


class SigningTests(unittest.TestCase):
    def setUp(self):
        self.now = dt.datetime(2026, 9, 25, tzinfo=dt.timezone.utc)
        self.certificate = b"synthetic certificate"
        self.fingerprint = hashlib.sha1(self.certificate).hexdigest().upper()
        self.identities = f'  1) {self.fingerprint} "Apple Development: Test"\n  1 valid identities found'
        self.profile = {
            "Platform": ["iOS"],
            "ExpirationDate": dt.datetime(2027, 9, 25),
            "ProvisionedDevices": ["device-udid"],
            "ApplicationIdentifierPrefix": ["OLDPREFIX"],
            "TeamIdentifier": ["NEWTEAM"],
            "DeveloperCertificates": [self.certificate],
            "Entitlements": {
                "application-identifier": "OLDPREFIX.dev.mel.*",
                "com.apple.developer.team-identifier": "NEWTEAM",
                "get-task-allow": True,
                "unrelated-capability": True,
            },
        }
        self.device = {
            "hardwareProperties": {
                "platform": "iOS", "reality": "physical", "deviceType": "iPhone", "udid": "device-udid",
            },
            "deviceProperties": {"osVersionNumber": "17.0", "developerModeStatus": "enabled"},
        }
        self.bundle_id = "dev.mel.example.006-frame-timing"

    def settings(self, requested=""):
        return device_runner.signing_settings(
            self.profile, self.identities, self.device, self.bundle_id, requested, self.now
        )

    def test_wildcard_expansion_preserves_distinct_prefix_and_team(self):
        identity, entitlements = self.settings()
        self.assertEqual(identity, self.fingerprint)
        self.assertEqual(entitlements, {
            "application-identifier": f"OLDPREFIX.{self.bundle_id}",
            "com.apple.developer.team-identifier": "NEWTEAM",
            "get-task-allow": True,
        })

    def test_explicit_app_id_matches_only_that_app(self):
        self.profile["Entitlements"]["application-identifier"] = f"OLDPREFIX.{self.bundle_id}"
        self.settings()
        self.bundle_id += "-other"
        with self.assertRaisesRegex(ValueError, "authorize BUNDLE_ID"):
            self.settings()

    def test_wildcard_does_not_authorize_another_namespace(self):
        self.bundle_id = "dev.melicious.example"
        with self.assertRaisesRegex(ValueError, "authorize BUNDLE_ID"):
            self.settings()

    def test_expired_profile_and_exact_expiry_are_rejected(self):
        for expiry in [dt.datetime(2025, 1, 1), self.now.replace(tzinfo=None)]:
            with self.subTest(expiry=expiry):
                self.profile["ExpirationDate"] = expiry
                with self.assertRaisesRegex(ValueError, "expired"):
                    self.settings()

    def test_device_must_be_in_profile_not_just_paired(self):
        self.profile["ProvisionedDevices"] = ["another-udid"]
        with self.assertRaisesRegex(ValueError, "not provisioned"):
            self.settings()

    def test_distribution_and_wrong_platform_profiles_are_rejected(self):
        original = copy.deepcopy(self.profile)
        for key, value, message in [
            ("Platform", ["OSX"], "iOS profile"),
            ("ProvisionedDevices", [], "not provisioned"),
            ("TeamIdentifier", ["OTHERTEAM"], "team identifier"),
            ("ApplicationIdentifierPrefix", ["OTHERPREFIX"], "App ID prefix"),
        ]:
            with self.subTest(key=key):
                self.profile = copy.deepcopy(original)
                self.profile[key] = value
                with self.assertRaisesRegex(ValueError, message):
                    self.settings()
        self.profile = original
        self.profile["Entitlements"]["get-task-allow"] = False
        with self.assertRaisesRegex(ValueError, "development profile"):
            self.settings()

    def test_missing_or_unauthorized_private_key_is_rejected(self):
        for identities in ["0 valid identities found", f'1) {"A" * 40} "Unrelated certificate"']:
            with self.subTest(identities=identities):
                self.identities = identities
                with self.assertRaisesRegex(ValueError, "0 matching"):
                    self.settings()

    def test_multiple_identities_require_explicit_selection(self):
        second_certificate = b"second synthetic certificate"
        second_hash = hashlib.sha1(second_certificate).hexdigest().upper()
        self.profile["DeveloperCertificates"].append(second_certificate)
        self.identities += f'\n  2) {second_hash} "Apple Development: Test"'
        with self.assertRaisesRegex(ValueError, "2 matching"):
            self.settings()
        self.assertEqual(self.settings(second_hash.lower())[0], second_hash)
        with self.assertRaisesRegex(ValueError, "0 matching"):
            self.settings("F" * 40)

    def test_device_prerequisites_are_checked(self):
        original = copy.deepcopy(self.device)
        for section, key, value, message in [
            ("hardwareProperties", "reality", "simulated", "physical"),
            ("hardwareProperties", "deviceType", "iPad", "iPhone only"),
            ("deviceProperties", "osVersionNumber", "16.7", "iOS 17"),
            ("deviceProperties", "developerModeStatus", "disabled", "enable Developer Mode"),
        ]:
            with self.subTest(key=key):
                self.device = copy.deepcopy(original)
                self.device[section][key] = value
                with self.assertRaisesRegex(ValueError, message):
                    self.settings()

    def test_invalid_bundle_id_is_rejected_before_deployment(self):
        for bundle_id in ["dev.mel.006_frame_timing", "dev.mel.*", "dev.mel.", "../somewhere"]:
            with self.subTest(bundle_id=bundle_id):
                self.bundle_id = bundle_id
                with self.assertRaisesRegex(ValueError, "dotted identifier"):
                    self.settings()


class CommandPrivacyTests(unittest.TestCase):
    def test_console_redaction_preserves_literal_arguments_without_a_shell(self):
        # Deliberately synthetic values; metacharacters must remain plain argv.
        command = ["xcrun", "--device", "test-device; echo unexpected", "--sign", "test-fingerprint"]
        diagnostics = io.StringIO()
        with patch.object(device_runner.subprocess, "run") as execute:
            with contextlib.redirect_stderr(diagnostics):
                device_runner.run(*command, capture=True)
        execute.assert_called_once_with(command, check=True, stdout=subprocess.PIPE)
        for private_value in (command[2], command[4]):
            self.assertNotIn(private_value, diagnostics.getvalue())
        self.assertEqual(diagnostics.getvalue().count("<redacted>"), 2)

    def test_failed_command_does_not_reintroduce_private_arguments(self):
        for flag in ("--device", "--sign"):
            with self.subTest(flag=flag):
                command = ["test-tool", flag, "private-test-value"]
                diagnostics = io.StringIO()
                failure = subprocess.CalledProcessError(1, command)
                with patch.object(device_runner.subprocess, "run", side_effect=failure):
                    with contextlib.redirect_stderr(diagnostics):
                        with self.assertRaises(RuntimeError) as raised:
                            device_runner.run(*command)
                self.assertNotIn(command[2], str(raised.exception))
                self.assertNotIn(command[2], diagnostics.getvalue())
                self.assertIn("exit code 1", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
