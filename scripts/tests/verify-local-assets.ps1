$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$runRoot = Join-Path $repo ("work/asset-evaluation/" + [Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $runRoot -Force | Out-Null
$inputPath = Join-Path $runRoot 'original.png'
[IO.File]::WriteAllBytes($inputPath, [Convert]::FromBase64String('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a6hQAAAAASUVORK5CYII='))
$originalHash = (Get-FileHash -LiteralPath $inputPath -Algorithm SHA256).Hash
$source = Join-Path $runRoot 'source'
New-Item -ItemType Directory -Path (Join-Path $source 'assets') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $source '.delivery/test/build') -Force | Out-Null
Set-Content -LiteralPath (Join-Path $source '.delivery/test/build/FixtureMod.dll') -Value 'isolated package fixture, not a compiled assembly' -Encoding UTF8
Set-Content -LiteralPath (Join-Path $source 'info.ini') -Value 'name=FixtureMod' -Encoding UTF8
Set-Content -LiteralPath (Join-Path $source 'README.md') -Value 'fixture only' -Encoding UTF8
Copy-Item -LiteralPath $inputPath -Destination (Join-Path $source 'assets/pixel.png')
Copy-Item -LiteralPath $inputPath -Destination (Join-Path $source 'preview.png')
$manifest = Join-Path $source '.delivery/test/resources.json'
'[{"source":"assets/pixel.png","target":"images/pixel.png"}]' | Set-Content -LiteralPath $manifest -Encoding UTF8
$package = Join-Path $source '.delivery/test/package'
Push-Location $source
try {
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $repo 'skills/games/duckov/duckov-mod/scripts/package.ps1') -ModName FixtureMod -BuildDirectory (Join-Path $source '.delivery/test/build') -SourceDirectory $source -PackageDirectory $package -ResourceManifest $manifest
    if ($LASTEXITCODE -ne 0) { throw '显式资源打包失败' }
    if ((Get-FileHash -LiteralPath (Join-Path $package 'images/pixel.png') -Algorithm SHA256).Hash -ne $originalHash) { throw '资源未正确进入交付目录' }
    '[{"source":"assets/pixel.png","target":"../escape.png"}]' | Set-Content -LiteralPath $manifest -Encoding UTF8
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $repo 'skills/games/duckov/duckov-mod/scripts/package.ps1') -ModName FixtureMod -BuildDirectory (Join-Path $source '.delivery/test/build') -SourceDirectory $source -PackageDirectory (Join-Path $source '.delivery/test/rejected') -ResourceManifest $manifest
    if ($LASTEXITCODE -eq 0 -or (Test-Path -LiteralPath (Join-Path $source '.delivery/test/escape.png'))) { throw '越界资源目标未被拒绝' }
} finally { Pop-Location }
Write-Output "通过：原始文件资源打包哈希及越界目标拒绝。夹具保留：$runRoot"
