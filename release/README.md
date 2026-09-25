# Release files

Built installers are stored here locally and published as assets of the matching
[GitHub release](../../../releases); the binaries themselves are not committed.

| File | Platform | Install |
|---|---|---|
| `OmniPack_<version>_x64_en-US.msi` | Windows 10/11 x64 | Double-click. Windows may show SmartScreen: choose "More info → Run anyway" (not code-signed yet). |
| `OmniPack_<version>_x64-setup.exe` | Windows 10/11 x64 | Alternative per-user installer (NSIS). |
| `OmniPack_<version>_android-universal.apk` | Android 7.0+ (arm64 phones, x86_64 emulators) | Copy to the phone and open it, allowing "install unknown apps". Signed with a test key. |
| `SHA256SUMS.txt` | all | Checksums: `sha256sum -c SHA256SUMS.txt` or `Get-FileHash <file>` |
