param([ValidateSet('desktop','check','smoke','bundle')][string]$Task = 'desktop')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$localNode = Get-ChildItem -LiteralPath (Join-Path $projectRoot '.tools') -Directory -Filter 'node-*-win-x64' -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
if ($localNode -and (Test-Path -LiteralPath (Join-Path $localNode.FullName 'node.exe'))) {
    $env:Path = "$($localNode.FullName);$env:Path"
}
$localCargo = Join-Path $projectRoot '.tools\cargo'
if (Test-Path -LiteralPath (Join-Path $localCargo 'bin\cargo.exe')) {
    $env:CARGO_HOME = $localCargo
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
    $env:Path = "$localCargo\bin;$env:Path"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Geliştirme için Rust ve Visual Studio C++ Build Tools gerekir. README.md dosyasına bakın.' }
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) { throw 'Geliştirme için Node.js ve pnpm gerekir.' }
switch ($Task) {
    'desktop' { pnpm desktop }
    'smoke' { cargo run -p aevra-core --example smoke }
    'bundle' { pnpm bundle:windows }
    'check' {
        pnpm build
        if ($LASTEXITCODE) { exit $LASTEXITCODE }
        pnpm test
        if ($LASTEXITCODE) { exit $LASTEXITCODE }
        cargo test -p aevra-core --locked
        if ($LASTEXITCODE) { exit $LASTEXITCODE }
        cargo check -p aevra-desktop --locked
    }
}
exit $LASTEXITCODE
