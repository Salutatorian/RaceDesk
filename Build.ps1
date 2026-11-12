$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    cargo test
    if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    python -m unittest discover -s tests -v
    if ($LASTEXITCODE -ne 0) { throw 'Collector tests failed' }
    python -m PyInstaller --noconfirm --onedir --name racedesk-collector --distpath build/dist --workpath build/work --specpath build --collect-all scrapling --collect-all curl_cffi --collect-all patchright --collect-all browserforge --collect-all apify_fingerprint_datapoints --exclude-module playwright collector/collect.py
    if ($LASTEXITCODE -ne 0) { throw 'Collector packaging failed' }
    New-Item -ItemType Directory -Force -Path app,app/collector | Out-Null
    Copy-Item target/release/racedesk.exe app/RaceDesk.exe -Force
    Copy-Item build/dist/racedesk-collector/* app/collector -Recurse -Force
    # HTTP-only collection does not launch the patchright Node driver.
    $nodeDriver = Join-Path $PSScriptRoot 'app\collector\_internal\patchright\driver\node.exe'
    if (Test-Path -LiteralPath $nodeDriver) { Remove-Item -LiteralPath $nodeDriver }
    & app/collector/racedesk-collector.exe build/packaged-check.json
    if ($LASTEXITCODE -ne 0) { throw 'Packaged live collection failed' }
} finally { Pop-Location }
