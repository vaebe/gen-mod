param(
    [Parameter(Mandatory=$true)][string]$ModName,
    [Parameter(Mandatory=$true)][string]$BuildDirectory,
    [Parameter(Mandatory=$true)][string]$SourceDirectory,
    [Parameter(Mandatory=$true)][string]$PackageDirectory
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
    Write-Output '交付文件已整理；请使用 prepare_delivery 打包并登记版本。'
    exit 0
} catch { Write-Error $_ -ErrorAction Continue; exit 1 }
