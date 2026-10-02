# Release files

Built installers are stored here locally and published as assets of the matching
[GitHub release](../../../releases); the binaries themselves are not committed.

| File | Platform | Install |
|---|---|---|
| `OmniPack_<version>_x64_en-US.msi` | Windows 10/11 x64 | Double-click. Windows may show SmartScreen: choose "More info → Run anyway" (not code-signed yet). |
| `OmniPack_<version>_x64-setup.exe` | Windows 10/11 x64 | Alternative per-user installer (NSIS). |
| `OmniPack_<version>_android-universal.apk` | Android 7.0+ (arm64 phones, x86_64 emulators) | Copy to the phone and open it, allowing "install unknown apps". Signed with a test key. |
| `omnipack-server_<version>_windows-x64.exe` | Windows x64 | The integration API as a standalone service (no install): `omnipack-server --api-key <key>`. See [docs/api.md](../docs/api.md). |
| `omnipack-server_<version>_linux-x64` | Linux x86_64 (glibc 2.35+: Ubuntu 22.04, Debian 12 or newer) | The same server: `chmod +x` and run, or build the Docker image from the repository. |
| `SHA256SUMS.txt` | all | Checksums: `sha256sum -c SHA256SUMS.txt` or `Get-FileHash <file>` |
