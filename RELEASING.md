# Publishing RaceDesk on GitHub

Upload **RaceDesk-Setup-0.1.0-x64.exe** as a GitHub Release asset. Users download and run that one file, then may delete it after installation. The installed app and uninstaller live independently in their LocalAppData folder.

The companion SHA256 file lets users verify the download. This build is unsigned; Windows may display an unknown-publisher warning. Public signing requires a signing identity owned by the publisher.

Extract the source archive into a GitHub repository. Do not commit app binaries, local preferences or development environments to normal source history; use Releases for installers. Include the third-party notices.

## Rebuild

1. Install Rust stable for Windows x64, Python 3.11+, and Inno Setup 6.7 or later.
2. Install collector dependencies: `python -m pip install "scrapling[fetchers]==0.5.1" pyinstaller`.
3. Run `powershell -ExecutionPolicy Bypass -File Build.ps1`.
4. Compile `installer/RaceDesk.iss` with Inno Setup's `ISCC.exe`; use `/O` to select the output folder.
5. Test the generated installer and native uninstaller on Windows. Sign before release if you have a publisher certificate.

Repository: https://github.com/Salutatorian/RaceDesk

Published downloads belong under https://github.com/Salutatorian/RaceDesk/releases. Commit application source and the README screenshot to the repository; attach the installer, source archive and checksums to the release.
