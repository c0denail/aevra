param([string]$Repository, [switch]$Upload)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$version = (Get-Content -LiteralPath 'package.json' -Raw | ConvertFrom-Json).version
$tag = "v$version"
$prefix = "Aevra-$version-windows-x64-setup"
$installer = Join-Path $projectRoot 'artifacts\installer'
$checksumFile = Join-Path $installer 'SHA256SUMS.txt'
$checks = @()
foreach ($line in Get-Content -LiteralPath $checksumFile) {
    if ($line -notmatch '^([a-f0-9]{64})  ([A-Za-z0-9.-]+)$') { throw 'Geçersiz kurulum özeti.' }
    $expectedHash, $name = $Matches[1], $Matches[2]
    if ($name -ne "$prefix.exe" -and $name -notmatch "^$([regex]::Escape($prefix))-[1-9][0-9]*\.bin$") { throw "Beklenmeyen dosya: $name" }
    if ($checks.Name -contains $name) { throw "Tekrarlanan dosya: $name" }
    $file = Get-Item -LiteralPath (Join-Path $installer $name)
    if ($file.Length -ge 2GB) { throw "GitHub 2 GiB sınırı aşılıyor: $name" }
    if ((Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedHash) { throw "Özet eşleşmiyor: $name" }
    $checks += [pscustomobject]@{ Name = $name; Path = $file.FullName; Size = $file.Length; Hash = $expectedHash }
}
$slices = @($checks | Where-Object Name -Like '*.bin')
if ($checks.Count -lt 2 -or -not ($checks.Name -contains "$prefix.exe")) { throw 'Kurulum EXE veya veri dilimleri eksik.' }
foreach ($number in 1..$slices.Count) {
    if ($checks.Name -notcontains "$prefix-$number.bin") { throw "Eksik kurulum dilimi: $number" }
}
$actualNames = @(Get-ChildItem -LiteralPath $installer -File | Where-Object { $_.Name -like "$prefix*" -and $_.Extension -in '.exe','.bin' })
if ($actualNames.Count -ne $checks.Count) { throw 'Kurulum klasöründe özette bulunmayan dilimler var.' }
$notes = Join-Path $projectRoot "docs\releases\$tag.md"
if (-not (Test-Path -LiteralPath $notes)) { throw "Sürüm açıklaması eksik: $notes" }
$checks += [pscustomobject]@{ Name = 'SHA256SUMS.txt'; Path = $checksumFile; Size = (Get-Item -LiteralPath $checksumFile).Length; Hash = (Get-FileHash -LiteralPath $checksumFile -Algorithm SHA256).Hash.ToLowerInvariant() }
$checks | Select-Object Name,Size | Format-Table -AutoSize
if (-not $Upload) { Write-Output 'Dosyalar doğrulandı; GitHub üzerinde değişiklik yapılmadı.'; return }
if ($Repository -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') { throw '-Repository hesap/depo belirtin.' }
$gh = Get-Command gh -ErrorAction SilentlyContinue
if (-not $gh) {
    $gh = Get-ChildItem -Path (Join-Path $projectRoot '.tools\gh_*\bin\gh.exe') -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1
}
if (-not $gh) { throw 'GitHub CLI kurulmalı ve gh auth login ile oturum açılmalı.' }
$ghPath = if ($gh.Source) { $gh.Source } else { $gh.FullName }
function Invoke-Gh {
    & $ghPath @args
    if ($LASTEXITCODE -ne 0) { throw "GitHub işlemi başarısız (kod $LASTEXITCODE). Aynı komutla tekrar denenebilir." }
}
$changes = git status --porcelain
if ($LASTEXITCODE -ne 0 -or $changes) { throw 'Önce kaynak değişikliklerini commit edin.' }
$commit = git rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'Commit bulunamadı.' }
Invoke-Gh repo view $Repository --json nameWithOwner,url
$remoteCommit = Invoke-Gh api "repos/$Repository/commits/$commit" --jq '.sha'
if ($remoteCommit -ne $commit) { throw 'Bu commit önce GitHub deposuna gönderilmeli.' }
$releaseText = & $ghPath release view $tag --repo $Repository --json isDraft 2>$null
if ($LASTEXITCODE -ne 0) {
    Invoke-Gh release create $tag --repo $Repository --target $commit --title "Aevra $version" --notes-file $notes --draft
}
$release = (Invoke-Gh api "repos/$Repository/releases/tags/$tag") | ConvertFrom-Json
if (-not $release.draft -or $release.target_commitish -ne $commit) { throw 'Sürüm yayımlanmış veya farklı commit ile oluşturulmuş; üzerine yazılmadı.' }
foreach ($file in $checks) {
    $existing = $release.assets | Where-Object name -EQ $file.Name | Select-Object -First 1
    if ($existing) {
        if ($existing.size -ne $file.Size -or $existing.digest -ne "sha256:$($file.Hash)") { throw "GitHub dosyası eşleşmiyor: $($file.Name). Taslakta inceleyin; otomatik silinmedi." }
        Write-Output "Zaten doğrulanmış: $($file.Name)"
    } else {
        Invoke-Gh release upload $tag $file.Path --repo $Repository
    }
}
$release = (Invoke-Gh api "repos/$Repository/releases/tags/$tag") | ConvertFrom-Json
if (@($release.assets).Count -ne $checks.Count) { throw 'Release dosya sayısı beklenenden farklı.' }
foreach ($file in $checks) {
    $asset = $release.assets | Where-Object name -EQ $file.Name | Select-Object -First 1
    if (-not $asset -or $asset.state -ne 'uploaded' -or $asset.size -ne $file.Size -or $asset.digest -ne "sha256:$($file.Hash)") { throw "Yükleme doğrulanamadı: $($file.Name)" }
}
Write-Output "Tüm sürüm dosyaları doğrulandı. Taslağı inceleyip yayımlayın: $($release.html_url)"
