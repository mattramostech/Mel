#!/usr/bin/env python3
"""Build, sign, install and launch an example on an explicitly selected iPhone."""

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shlex
import shutil
import subprocess
import sys
import tempfile


def permits(pattern, value):
    """Apple App IDs allow a terminal wildcard, not arbitrary glob syntax."""
    return pattern == value or (
        pattern.endswith("*")
        and pattern.count("*") == 1
        and value.startswith(pattern[:-1])
    )


def signing_settings(profile, identities, device, bundle_id, requested="", now=None):
    """Validate before deployment; never infer an App ID prefix from the team."""
    if not re.fullmatch(r"[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+", bundle_id):
        raise ValueError("BUNDLE_ID must be a dotted identifier without underscores")
    hardware = device["hardwareProperties"]
    properties = device["deviceProperties"]
    if hardware["platform"] != "iOS" or hardware["reality"] != "physical":
        raise ValueError("DEVICE_ID must select a physical iOS device")
    if hardware["deviceType"] != "iPhone":
        raise ValueError("this bundle supports iPhone only")
    if int(properties["osVersionNumber"].split(".")[0]) < 17:
        raise ValueError("the devicectl flow requires iOS 17 or newer")
    if properties["developerModeStatus"] != "enabled":
        raise ValueError("enable Developer Mode in iPhone Settings > Privacy & Security")
    if "iOS" not in profile.get("Platform", []):
        raise ValueError("PROVISIONING_PROFILE must be an iOS profile")
    expiry = profile["ExpirationDate"].replace(tzinfo=dt.timezone.utc)
    if expiry <= (now or dt.datetime.now(dt.timezone.utc)):
        raise ValueError("provisioning profile expired; renew it in Xcode")
    if hardware["udid"] not in profile.get("ProvisionedDevices", []):
        raise ValueError("selected iPhone UDID is not provisioned by this profile")
    allowed = profile["Entitlements"]
    if allowed.get("get-task-allow") is not True:
        raise ValueError("use a development profile with get-task-allow enabled")
    prefix, separator, _ = allowed["application-identifier"].partition(".")
    if not separator or prefix not in profile["ApplicationIdentifierPrefix"]:
        raise ValueError("profile has an inconsistent App ID prefix")
    app_id = f"{prefix}.{bundle_id}"
    if not permits(allowed["application-identifier"], app_id):
        raise ValueError("profile does not authorize BUNDLE_ID; choose a matching profile or ID")
    team = allowed["com.apple.developer.team-identifier"]
    if team not in profile["TeamIdentifier"]:
        raise ValueError("profile has an inconsistent team identifier")

    # A valid keychain identity has the private key. Intersect its fingerprint
    # with the profile's certificate list, not a possibly ambiguous display name.
    installed = set(re.findall(r'\) ([0-9A-Fa-f]{40}) "', identities.upper()))
    authorized = {hashlib.sha1(cert).hexdigest().upper() for cert in profile["DeveloperCertificates"]}
    candidates = installed & authorized
    if requested:
        candidates &= {requested.upper()}
    if len(candidates) != 1:
        raise ValueError(
            f"found {len(candidates)} matching signing identities; install the profile's "
            "development certificate/private key, or set SIGNING_IDENTITY to its SHA-1"
        )
    # A profile is an allowlist, not the app's requested capabilities. Mel needs
    # only these development entitlements; do not copy unrelated capabilities.
    return candidates.pop(), {
        "application-identifier": app_id,
        "com.apple.developer.team-identifier": team,
        "get-task-allow": True,
    }


def bundle_info(example, bundle_id):
    return {
        "CFBundleIdentifier": bundle_id,
        "CFBundleExecutable": example,
        "CFBundleName": example,
        "CFBundlePackageType": "APPL",
        "CFBundleVersion": "1",
        "CFBundleShortVersionString": "0.1.0",
        "CFBundleSupportedPlatforms": ["iPhoneOS"],
        "MinimumOSVersion": "15.0",
        "UIDeviceFamily": [1],
        "UIRequiredDeviceCapabilities": ["arm64"],
        "UILaunchScreen": {},
        "UISupportedInterfaceOrientations": [
            "UIInterfaceOrientationPortrait",
            "UIInterfaceOrientationLandscapeLeft",
            "UIInterfaceOrientationLandscapeRight",
        ],
    }


def display_command(command):
    """Keep device selectors and certificate fingerprints out of our diagnostics."""
    displayed = []
    private_value = False
    for arg in command:
        if private_value:
            displayed.append("<redacted>")
            private_value = False
        else:
            displayed.append(arg)
            private_value = arg in ("--device", "--sign")
    return shlex.join(displayed)


def run(*args, capture=False, **kwargs):
    command = [str(arg) for arg in args]
    displayed = display_command(command)
    print("+ " + displayed, file=sys.stderr, flush=True)
    try:
        return subprocess.run(command, check=True, stdout=subprocess.PIPE if capture else None, **kwargs)
    except subprocess.CalledProcessError as error:
        # CalledProcessError.__str__ includes the original, unredacted argv.
        raise RuntimeError(f"{displayed} failed (exit code {error.returncode})") from None


def main():
    parser = argparse.ArgumentParser(description=__doc__, epilog=(
        "Required environment: DEVICE_ID (devicectl identifier), PROVISIONING_PROFILE (path). "
        "Optional: BUNDLE_ID, SIGNING_IDENTITY (SHA-1), BUILD_MODE (debug or release). "
        "See examples/README.md for one-time signing setup. Ctrl-C stops the device app."
    ))
    parser.add_argument("example", help="e.g. 006_frame_timing")
    args = parser.parse_args()
    example = args.example
    if not re.fullmatch(r"[0-9]{3}_[a-z0-9_]+", example):
        parser.error("expected a numbered example such as 006_frame_timing")
    root = Path(__file__).resolve().parent.parent
    if not (root / "examples" / f"{example}.rs").is_file():
        parser.error(f"unknown example: {example}")
    device_id = os.environ.get("DEVICE_ID", "")
    profile_path = os.environ.get("PROVISIONING_PROFILE", "")
    if not device_id or not profile_path:
        parser.error("set DEVICE_ID and PROVISIONING_PROFILE; see examples/README.md")
    profile_path = Path(profile_path).expanduser().resolve()
    if not profile_path.is_file():
        parser.error("PROVISIONING_PROFILE does not name a file")
    mode = os.environ.get("BUILD_MODE", "debug")
    if mode not in ("debug", "release"):
        parser.error("BUILD_MODE must be debug or release")
    bundle_id = os.environ.get("BUNDLE_ID", f"dev.mel.example.{example.replace('_', '-')}")

    with tempfile.TemporaryDirectory(prefix="mel-device-") as scratch:
        scratch = Path(scratch)
        device_json = scratch / "device.json"
        run("xcrun", "devicectl", "device", "info", "details", "--device", device_id,
            "--timeout", "30", "--quiet", "--json-output", device_json)
        device = json.loads(device_json.read_text())["result"]
        # Snapshot the supplied profile so validation and embedding use the same bytes.
        embedded = scratch / "embedded.mobileprovision"
        shutil.copyfile(profile_path, embedded)
        decoded = run("security", "cms", "-D", "-i", embedded, capture=True).stdout
        profile = plistlib.loads(decoded)
        identities = run("security", "find-identity", "-v", "-p", "codesigning", capture=True).stdout.decode()
        identity, entitlements = signing_settings(
            profile, identities, device, bundle_id, os.environ.get("SIGNING_IDENTITY", "")
        )
        print(f"Deploying {example} ({mode}) to {device['hardwareProperties']['marketingName']} "
              f"/ iOS {device['deviceProperties']['osVersionNumber']}", flush=True)
        target = "aarch64-apple-ios"
        # A fixed output directory avoids ambiguity from CARGO_TARGET_DIR overrides.
        target_dir = root / "target"
        run("cargo", "build", "--locked", "--target", target, "--target-dir", target_dir,
            "-p", "mel-platform", "--example", example,
            *(["--release"] if mode == "release" else []), cwd=root)
        app_dir = target_dir / "device" / f"{example}.app"
        if app_dir.exists():
            shutil.rmtree(app_dir)
        app_dir.mkdir(parents=True)
        shutil.copy2(target_dir / target / mode / "examples" / example, app_dir / example)
        shutil.copyfile(embedded, app_dir / embedded.name)
        (app_dir / "Info.plist").write_bytes(plistlib.dumps(bundle_info(example, bundle_id)))
        entitlements_path = scratch / "entitlements.plist"
        entitlements_path.write_bytes(plistlib.dumps(entitlements))
        run("codesign", "--force", "--sign", identity, "--entitlements", entitlements_path,
            "--generate-entitlement-der", "--timestamp=none", app_dir)
        run("codesign", "--verify", "--strict", "--verbose=2", app_dir)
        run("xcrun", "devicectl", "device", "install", "app", "--device", device_id,
            "--timeout", "120", app_dir)
    # Metadata is already cleaned up while the console stays attached.
    print("Launching; Ctrl-C stops the device app.", flush=True)
    run("xcrun", "devicectl", "device", "process", "launch", "--device", device_id,
        "--terminate-existing", "--console", bundle_id)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
    except (OSError, ValueError, KeyError, TypeError, RuntimeError) as error:
        print(f"error: {error}", file=sys.stderr)
        sys.exit(1)
