param(
    [Parameter(Mandatory=$true)][string]$ModName,
    [Parameter(Mandatory=$true)][string]$BuildDirectory,
    [Parameter(Mandatory=$true)][string]$SourceDirectory,
    [Parameter(Mandatory=$true)][string]$PackageDirectory,
    [string]$ResourceManifest
)
$ErrorActionPreference = 'Stop'
try {
    if ($ModName -notmatch '^[A-Za-z_][A-Za-z0-9_]*$') { throw 'ModName 必须是合法 C# 标识符。' }
    $root = [IO.Path]::GetFullPath((Get-Location).Path).TrimEnd('\') + '\'
    $target = [IO.Path]::GetFullPath($PackageDirectory)
    if (-not $target.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) { throw '交付目录必须位于当前 source 内。' }
    if (Test-Path -LiteralPath $target) { throw '交付目录已存在，不能复用旧产物。' }
    $dll = Join-Path $BuildDirectory "$ModName.dll"
    if (-not (Test-Path -LiteralPath $dll -PathType Leaf)) { throw '缺少本次构建的 MOD DLL。' }
    $info = Join-Path $SourceDirectory 'info.ini'
    if (-not ((Get-Content -LiteralPath $info) -contains "name=$ModName")) { throw 'info.ini 的 name 与 DLL 名称不一致。' }
    $sourceBase = [IO.Path]::GetFullPath($SourceDirectory).TrimEnd('\')
    if ($sourceBase -ne $root.TrimEnd('\') -and -not ($sourceBase + '\').StartsWith($root,[StringComparison]::OrdinalIgnoreCase)) { throw '资源来源必须在当前 source 中。' }
    function Assert-PlainPath([string]$Path,[string]$Stop) {
        $cursor = $Path
        while ($true) {
            $item = Get-Item -LiteralPath $cursor -Force
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw '资源路径不接受重解析点。' }
            if ($cursor -eq $Stop) { break }
            $next = [IO.Path]::GetDirectoryName($cursor)
            if (-not $next -or $next -eq $cursor) { throw '资源路径不属于 source。' }
            $cursor = $next
        }
    }
    function Assert-RelativeResource([string]$Value) {
        if ([string]::IsNullOrWhiteSpace($Value) -or [IO.Path]::IsPathRooted($Value) -or $Value.Contains('\') -or $Value.Contains(':')) { throw '资源清单需使用 / 分隔的相对路径。' }
        foreach ($part in $Value.Split('/')) {
            if (-not $part -or $part -eq '.' -or $part -eq '..' -or $part.EndsWith('.') -or $part.EndsWith(' ') -or $part.IndexOfAny([IO.Path]::GetInvalidFileNameChars()) -ge 0) { throw '资源清单包含无效路径段。' }
        }
    }
    $resourceEntries = @()
    $targetNames = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    foreach ($name in @("$ModName.dll",'info.ini','README.md','preview.png')) { [void]$targetNames.Add($name) }
    if ($ResourceManifest) {
        $manifestPath = [IO.Path]::GetFullPath($ResourceManifest)
        if (-not $manifestPath.StartsWith($root,[StringComparison]::OrdinalIgnoreCase)) { throw '资源清单必须保存在当前 source 中。' }
        Assert-PlainPath $manifestPath $root.TrimEnd('\')
        if ((Get-Item -LiteralPath $manifestPath).Length -gt 1048576) { throw '资源清单超过 1 MiB。' }
        $raw = Get-Content -LiteralPath $manifestPath -Raw
        if (-not $raw.TrimStart().StartsWith('[')) { throw '资源清单必须为 JSON 数组。' }
        $resourceEntries = @(ConvertFrom-Json -InputObject $raw)
        if ($resourceEntries.Count -gt 1000) { throw '资源清单超过 1000 项。' }
        foreach ($entry in $resourceEntries) {
            if ($entry.source -isnot [string] -or $entry.target -isnot [string]) { throw '资源清单需要 source 和 target 字符串。' }
            Assert-RelativeResource $entry.source
            Assert-RelativeResource $entry.target
            if (-not $targetNames.Add($entry.target)) { throw '资源清单目标重复或覆盖固定交付文件。' }
            $resource = [IO.Path]::GetFullPath((Join-Path $sourceBase $entry.source))
            Assert-PlainPath $resource $sourceBase
            if (-not (Test-Path -LiteralPath $resource -PathType Leaf)) { throw '资源源文件不存在。' }
        }
    }
    New-Item -ItemType Directory -Path $target | Out-Null
    Copy-Item -LiteralPath $dll -Destination (Join-Path $target "$ModName.dll")
    Copy-Item -LiteralPath $info -Destination (Join-Path $target 'info.ini')
    Copy-Item -LiteralPath (Join-Path $SourceDirectory 'README.md') -Destination (Join-Path $target 'README.md')
    $preview = Join-Path $SourceDirectory 'preview.png'
    if (Test-Path -LiteralPath $preview -PathType Leaf) { Copy-Item -LiteralPath $preview -Destination (Join-Path $target 'preview.png') }
    else {
        # 自行绘制中性预览标识，无游戏素材；不将参考程序集复制到交付目录。
        Add-Type -AssemblyName System.Drawing
        $bitmap = New-Object System.Drawing.Bitmap 256,256
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $graphics.Clear([System.Drawing.Color]::FromArgb(32,33,36))
        $font = New-Object System.Drawing.Font 'Segoe UI',24
        $graphics.DrawString('MOD', $font, [System.Drawing.Brushes]::White, 74,96)
        $bitmap.Save((Join-Path $target 'preview.png'), [System.Drawing.Imaging.ImageFormat]::Png)
        $font.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
    }
    foreach ($entry in $resourceEntries) {
        $destination = Join-Path $target $entry.target
        $parent = [IO.Path]::GetDirectoryName($destination)
        if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
        Copy-Item -LiteralPath (Join-Path $sourceBase $entry.source) -Destination $destination
    }
    Write-Output '交付文件已整理；请使用 prepare_delivery 打包并登记版本。'
    exit 0
} catch { Write-Error $_ -ErrorAction Continue; exit 1 }
