#!/usr/bin/env python3
"""Check an unsigned LightCraft .ipa (from ios-ipa.sh) before anyone sideloads it.

    verify-ipa.py LightCraft-iOS-….ipa [--bundle-id ID] [--json report.json]

Fails (exit 1) on anything that makes the package unusable for an iPhone or unsafe to hand out:
not exactly one Payload/<name>.app, a missing or inconsistent Info.plist, an executable that is not
an arm64 iPhoneOS (device, not Simulator) Mach-O, a deployment target above MinimumOSVersion, a
lost iOS 27 scene manifest, a missing compiled icon, or anything that looks like signing material
or a secret (provisioning profiles, certificates, private keys, entitlements files).
Warnings don't fail the check. Pure standard library, so it runs on macOS and Linux alike.
"""

from __future__ import annotations

import argparse
import json
import plistlib
import re
import stat
import struct
import sys
import zipfile

MH_MAGIC_64 = 0xFEEDFACF
FAT_MAGIC = 0xCAFEBABE
FAT_MAGIC_64 = 0xCAFEBABF
CPU_TYPE_ARM64 = 0x0100000C
MH_EXECUTE = 0x2
LC_ENCRYPTION_INFO_64 = 0x2C
LC_VERSION_MIN_IPHONEOS = 0x25
LC_BUILD_VERSION = 0x32
LC_CODE_SIGNATURE = 0x1D
PLATFORMS = {1: "macos", 2: "ios", 3: "tvos", 4: "watchos", 6: "maccatalyst", 7: "ios-simulator"}

SECRET_NAMES = re.compile(r"(\.mobileprovision|\.provisionprofile|\.p12|\.pfx|\.cer|\.der|\.pem|\.key|\.entitlements|\.xcent)$", re.I)
SECRET_TEXT = re.compile(rb"-----BEGIN [A-Z ]*PRIVATE KEY-----|<key>application-identifier</key>|<key>com\.apple\.developer\.team-identifier</key>")
MAX_SCAN = 4 * 1024 * 1024  # bytes of each non-executable file scanned for key material


class Report:
    def __init__(self) -> None:
        self.errors: list[str] = []
        self.warnings: list[str] = []
        self.facts: dict[str, object] = {}

    def error(self, msg: str) -> None:
        self.errors.append(msg)

    def warn(self, msg: str) -> None:
        self.warnings.append(msg)


def version_tuple(text: str) -> tuple[int, ...] | None:
    if not re.fullmatch(r"\d+(\.\d+){0,2}", text or ""):
        return None
    return tuple(int(p) for p in text.split("."))


def decode_version(v: int) -> str:
    return f"{v >> 16}.{(v >> 8) & 0xFF}.{v & 0xFF}"


def thin_arm64(data: bytes, rep: Report) -> bytes | None:
    """The arm64 slice of a (possibly universal) Mach-O, or None."""
    if len(data) < 8:
        rep.error("executable: too small to be a Mach-O")
        return None
    (magic_be,) = struct.unpack_from(">I", data, 0)
    if magic_be in (FAT_MAGIC, FAT_MAGIC_64):
        (count,) = struct.unpack_from(">I", data, 4)
        is64 = magic_be == FAT_MAGIC_64
        entry = 32 if is64 else 20
        archs = []
        for i in range(min(count, 32)):
            off = 8 + i * entry
            if off + entry > len(data):
                break
            if is64:
                cpu, _sub, offset, size = struct.unpack_from(">iiQQ", data, off)
            else:
                cpu, _sub, offset, size = struct.unpack_from(">iiII", data, off)
            archs.append(cpu)
            if cpu == CPU_TYPE_ARM64 and offset + size <= len(data):
                rep.facts["universal_slices"] = len(archs)
                return data[offset : offset + size]
        rep.error(f"executable: universal binary without an arm64 slice (cpu types {archs})")
        return None
    return data


def check_macho(data: bytes, min_os: tuple[int, ...] | None, rep: Report) -> None:
    data = thin_arm64(data, rep)
    if data is None:
        return
    if len(data) < 32:
        rep.error("executable: truncated Mach-O header")
        return
    magic, cpu, _sub, filetype, ncmds, sizeofcmds, _flags, _res = struct.unpack_from("<IiiIIIII", data, 0)
    if magic != MH_MAGIC_64:
        rep.error(f"executable: not a 64-bit little-endian Mach-O (magic {magic:#x})")
        return
    if cpu != CPU_TYPE_ARM64:
        rep.error(f"executable: CPU type {cpu:#x}, expected arm64 ({CPU_TYPE_ARM64:#x})")
    if filetype != MH_EXECUTE:
        rep.error(f"executable: Mach-O file type {filetype}, expected MH_EXECUTE")
    platform = None
    minos = None
    signed = False
    off = 32
    end = min(len(data), 32 + sizeofcmds)
    for _ in range(min(ncmds, 4096)):
        if off + 8 > end:
            break
        cmd, cmdsize = struct.unpack_from("<II", data, off)
        if cmdsize < 8:
            rep.error("executable: malformed load command")
            break
        if cmd == LC_BUILD_VERSION and off + 16 <= end:
            platform, minos = struct.unpack_from("<II", data, off + 8)
        elif cmd == LC_VERSION_MIN_IPHONEOS and off + 12 <= end:
            platform = 2
            (minos,) = struct.unpack_from("<I", data, off + 8)
        elif cmd == LC_ENCRYPTION_INFO_64 and off + 20 <= end:
            (cryptid,) = struct.unpack_from("<I", data, off + 16)
            if cryptid:
                rep.error("executable: encrypted (an App Store binary), cannot be re-signed")
        elif cmd == LC_CODE_SIGNATURE:
            signed = True
        off += cmdsize
    rep.facts["arch"] = "arm64" if cpu == CPU_TYPE_ARM64 else hex(cpu)
    rep.facts["platform"] = PLATFORMS.get(platform, str(platform))
    rep.facts["linker_signature"] = signed
    if platform != 2:
        what = "an iOS Simulator build" if platform == 7 else f"platform {PLATFORMS.get(platform, platform)}"
        rep.error(f"executable: {what}, expected iphoneos (device)")
    if minos is not None:
        rep.facts["binary_min_ios"] = decode_version(minos)
        bin_min = version_tuple(decode_version(minos))
        if min_os and bin_min and bin_min > min_os + (0,) * (3 - len(min_os)):
            rep.error(f"executable needs iOS {decode_version(minos)} but MinimumOSVersion says {'.'.join(map(str, min_os))}")
    else:
        rep.warn("executable: no deployment target load command")


def verify(path: str, bundle_id: str | None) -> Report:
    rep = Report()
    try:
        zf = zipfile.ZipFile(path)
    except (OSError, zipfile.BadZipFile) as e:
        rep.error(f"not a readable zip: {e}")
        return rep
    with zf:
        bad = zf.testzip()
        if bad:
            rep.error(f"corrupt zip member: {bad}")
        names = [i.filename for i in zf.infolist()]
        apps = set()
        for n in names:
            if n.startswith("/") or ".." in n.split("/"):
                rep.error(f"unsafe path in archive: {n}")
            if n.startswith("__MACOSX/") or n.endswith(".DS_Store"):
                rep.warn(f"stray macOS metadata: {n}")
                continue
            parts = n.split("/")
            if parts[0] != "Payload":
                rep.error(f"entry outside Payload/: {n}")
                continue
            if len(parts) >= 2 and parts[1]:
                if not parts[1].endswith(".app"):
                    rep.error(f"Payload/ holds something other than an .app: {n}")
                else:
                    apps.add(parts[1])
        if len(apps) != 1:
            rep.error(f"expected exactly one Payload/*.app, found {sorted(apps) or 'none'}")
            return rep
        app = f"Payload/{apps.pop()}/"
        rep.facts["app"] = app
        app_files = {n[len(app):]: zf.getinfo(n) for n in names if n.startswith(app) and not n.endswith("/")}

        info = app_files.get("Info.plist")
        if info is None:
            rep.error("Info.plist missing")
            return rep
        try:
            pl = plistlib.loads(zf.read(info))
        except Exception as e:  # noqa: BLE001 - any parse failure is the finding
            rep.error(f"Info.plist unreadable: {e}")
            return rep

        def need(key: str, typ: type) -> object | None:
            v = pl.get(key)
            if not isinstance(v, typ) or v in ("", [], {}):
                rep.error(f"Info.plist: {key} missing or not a {typ.__name__}")
                return None
            return v

        exe = need("CFBundleExecutable", str)
        bid = need("CFBundleIdentifier", str)
        short = need("CFBundleShortVersionString", str)
        build = need("CFBundleVersion", str)
        min_os_s = need("MinimumOSVersion", str)
        if pl.get("CFBundlePackageType") != "APPL":
            rep.error("Info.plist: CFBundlePackageType is not APPL")
        if bid and not re.fullmatch(r"[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+", bid):
            rep.error(f"Info.plist: CFBundleIdentifier '{bid}' is not a reverse-DNS id")
        if bundle_id and bid != bundle_id:
            rep.error(f"Info.plist: CFBundleIdentifier '{bid}', expected '{bundle_id}'")
        if short and not version_tuple(short):
            rep.error(f"Info.plist: CFBundleShortVersionString '{short}' is not 1-3 numbers")
        if build and not version_tuple(build):
            rep.error(f"Info.plist: CFBundleVersion '{build}' is not 1-3 numbers")
        min_os = version_tuple(min_os_s) if min_os_s else None
        if min_os_s and not min_os:
            rep.error(f"Info.plist: MinimumOSVersion '{min_os_s}' is not a version")
        platforms = pl.get("CFBundleSupportedPlatforms")
        if platforms is not None and "iPhoneOS" not in platforms:
            rep.error(f"Info.plist: CFBundleSupportedPlatforms {platforms} lacks iPhoneOS")
        if 1 not in (pl.get("UIDeviceFamily") or []):
            rep.error("Info.plist: UIDeviceFamily does not include iPhone (1)")

        scene = pl.get("UIApplicationSceneManifest") or {}
        cfgs = (scene.get("UISceneConfigurations") or {}).get("UIWindowSceneSessionRoleApplication") or []
        if not any(isinstance(c, dict) and c.get("UISceneDelegateClassName") for c in cfgs):
            rep.error("Info.plist: UIApplicationSceneManifest with a UISceneDelegateClassName is missing (iOS 27 kills the app without it)")
        if "UILaunchScreen" not in pl and "UILaunchStoryboardName" not in pl:
            rep.error("Info.plist: no launch screen (UILaunchScreen); iOS would letterbox the app")
        if "CFBundleIcons" not in pl:
            rep.error("Info.plist: CFBundleIcons missing (actool's partial plist not merged)")
        if "Assets.car" not in app_files:
            rep.error("Assets.car missing (app icon not compiled)")

        usage = sorted(k for k in pl if k.endswith("UsageDescription"))
        rep.facts["usage_descriptions"] = usage
        for k in usage:
            if k in ("NSPhotoLibraryUsageDescription", "NSPhotoLibraryAddUsageDescription"):
                rep.warn(f"Info.plist: {k} is set; PHPicker and the share sheet don't need photo library access")

        if exe:
            ei = app_files.get(exe)
            if ei is None:
                rep.error(f"executable '{exe}' (CFBundleExecutable) is not in the .app")
            else:
                mode = (ei.external_attr >> 16) & 0xFFFF
                if mode and not mode & stat.S_IXUSR:
                    rep.error(f"executable '{exe}' lost its execute permission in the zip")
                elif not mode:
                    rep.warn(f"executable '{exe}': the zip records no Unix permissions")
                rep.facts["executable_mb"] = round(ei.file_size / 1e6, 1)
                check_macho(zf.read(ei), min_os, rep)

        for rel, zi in app_files.items():
            base = rel.rsplit("/", 1)[-1]
            if SECRET_NAMES.search(base):
                rep.error(f"signing material or key file in the bundle: {rel}")
            if rel.startswith("_CodeSignature/"):
                rep.warn(f"bundle carries a code signature ({rel}); the sideloading tool replaces it")
            if rel != exe and zi.file_size <= MAX_SCAN and SECRET_TEXT.search(zf.read(zi)):
                rep.error(f"{rel} contains a private key or entitlements")

        rep.facts.update(
            bundle_id=bid, version=short, build=build, min_ios=min_os_s,
            commit=pl.get("LCBuildCommit"), files=len(app_files),
            ipa_mb=round(sum(i.compress_size for i in zf.infolist()) / 1e6, 1),
        )
    return rep


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("ipa")
    ap.add_argument("--bundle-id", help="fail unless CFBundleIdentifier is this")
    ap.add_argument("--json", help="also write the report here")
    args = ap.parse_args()
    rep = verify(args.ipa, args.bundle_id)
    for k, v in rep.facts.items():
        print(f"  {k}: {v}")
    for w in rep.warnings:
        print(f"WARNING: {w}")
    for e in rep.errors:
        print(f"ERROR: {e}")
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump({"ok": not rep.errors, "errors": rep.errors, "warnings": rep.warnings, "facts": rep.facts}, f, indent=2)
    print("IPA OK" if not rep.errors else f"IPA FAILED: {len(rep.errors)} error(s)")
    return 0 if not rep.errors else 1


if __name__ == "__main__":
    sys.exit(main())
