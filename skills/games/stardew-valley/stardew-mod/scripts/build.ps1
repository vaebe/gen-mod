param(
    [Parameter(Mandatory=$true)][ValidateSet('code','content-pack')][string]$Kind,
    [Parameter(Mandatory=$true)][string]$ModDirectory,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [string]$ProjectPath,
    [string]$ReferenceDirectory,
    [string]$ResourceManifest
)
$ErrorActionPreference = 'Stop'
try {
    $sourceRoot = [IO.Path]::GetFullPath((Get-Location).Path).TrimEnd('\')
    $sourcePrefix = $sourceRoot + '\'
    function Resolve-SourcePath([string]$Value) {
        $full = [IO.Path]::GetFullPath($Value)
        if ($full -ne $sourceRoot -and -not $full.StartsWith($sourcePrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw '工程、资源与输出路径必须位于当前 source 目录内。'
        }
        $cursor = $full
        while ($true) {
            if (Test-Path -LiteralPath $cursor) {
                $item = Get-Item -LiteralPath $cursor -Force
                if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw '工程与资源路径不接受重解析点。' }
            }
            if ($cursor -eq $sourceRoot) { break }
            $cursor = [IO.Path]::GetDirectoryName($cursor)
            if (-not $cursor) { throw '路径不属于当前 source。' }
        }
        return $full
    }
    function Assert-Relative([string]$Value) {
        if ([string]::IsNullOrWhiteSpace($Value) -or [IO.Path]::IsPathRooted($Value) -or $Value.Contains('\') -or $Value.Contains(':')) {
            throw '交付资源必须使用 / 分隔的相对路径。'
        }
        foreach ($part in $Value.Split('/')) {
            if (-not $part -or $part -eq '.' -or $part -eq '..' -or $part.EndsWith('.') -or $part.EndsWith(' ') -or $part.IndexOfAny([IO.Path]::GetInvalidFileNameChars()) -ge 0) {
                throw '交付资源包含无效路径段。'
            }
        }
    }
    function Read-JsonFile([string]$Path) {
        if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "缺少 JSON 文件：$Path" }
        if ((Get-Item -LiteralPath $Path).Length -gt 10MB) { throw "JSON 文件超过 10 MiB：$Path" }
        return (Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json)
    }
    $modRoot = Resolve-SourcePath $ModDirectory
    if (-not (Test-Path -LiteralPath $modRoot -PathType Container)) { throw 'MOD 工程目录不存在。' }
    $output = Resolve-SourcePath $OutputDirectory
    if (Test-Path -LiteralPath $output) { throw '输出目录已存在，请使用本任务的全新输出目录。' }
    $manifestPath = Resolve-SourcePath (Join-Path $modRoot 'manifest.json')
    $readmePath = Resolve-SourcePath (Join-Path $modRoot 'README.md')
    $manifest = Read-JsonFile $manifestPath
    foreach ($field in @('Name','Author','Version','Description','UniqueID','MinimumApiVersion')) {
        if ($manifest.$field -isnot [string] -or [string]::IsNullOrWhiteSpace($manifest.$field)) { throw "manifest.json 缺少有效的 $field。" }
    }
    if ($manifest.UniqueID -notmatch '^[A-Za-z0-9_]+\.[A-Za-z0-9_.-]+$') { throw 'UniqueID 必须为稳定的作者.模组标识。' }
    if ($manifest.UniqueID -like 'YourName.*') { throw '请先替换模板的作者与 UniqueID。' }
    if (-not (Test-Path -LiteralPath $readmePath -PathType Leaf)) { throw '缺少面向玩家的 README.md。' }
    $fixed = @('manifest.json','README.md')
    $referenceNames = @('StardewModdingAPI.dll','Stardew Valley.dll','StardewValley.GameData.dll','MonoGame.Framework.dll','xTile.dll','SMAPI.Toolkit.CoreInterfaces.dll')
    if ($Kind -eq 'content-pack') {
        if ($manifest.EntryDll -or $manifest.ContentPackFor.UniqueID -ne 'Pathoschild.ContentPatcher') { throw '内容包必须声明 ContentPackFor 为 Pathoschild.ContentPatcher，且不含 EntryDll。' }
        $content = Read-JsonFile (Resolve-SourcePath (Join-Path $modRoot 'content.json'))
        $formatVersion = $null
        if ($content.Format -isnot [string] -or -not [version]::TryParse($content.Format, [ref]$formatVersion)) { throw 'content.json 缺少有效的 Format。' }
        if ($content.Changes -isnot [array] -or $content.Changes.Count -eq 0) { throw '内容包没有 Changes，不能交付空模板。' }
        foreach ($change in $content.Changes) {
            if ($change.Action -isnot [string] -or [string]::IsNullOrWhiteSpace($change.Action)) { throw 'Changes 中缺少 Action。' }
        }
        $fixed += 'content.json'
    } else {
        if ($manifest.ContentPackFor -or $manifest.EntryDll -isnot [string] -or $manifest.EntryDll -notmatch '^[A-Za-z_][A-Za-z0-9_.-]*\.dll$') { throw '代码 MOD 必须声明单文件 EntryDll，且不含 ContentPackFor。' }
        if ($referenceNames -contains $manifest.EntryDll) { throw 'MOD DLL 不得使用游戏或 SMAPI 程序集名称。' }
        if (-not $ProjectPath -or -not $ReferenceDirectory) { throw 'C# 构建需要 ProjectPath 和 ReferenceDirectory。' }
        $project = Resolve-SourcePath $ProjectPath
        if (-not $project.StartsWith($modRoot.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetExtension($project) -ne '.csproj' -or -not (Test-Path -LiteralPath $project -PathType Leaf)) { throw 'C# 工程必须位于 ModDirectory 中。' }
        $references = [IO.Path]::GetFullPath($ReferenceDirectory)
        foreach ($name in $referenceNames) {
            $reference = Join-Path $references $name
            if ($name -eq 'SMAPI.Toolkit.CoreInterfaces.dll' -and -not (Test-Path -LiteralPath $reference -PathType Leaf)) {
                $reference = Join-Path $references 'smapi-internal/SMAPI.Toolkit.CoreInterfaces.dll'
            }
            if (-not (Test-Path -LiteralPath $reference -PathType Leaf)) { throw "缺少合法参考程序集：$name。源码已保留。" }
        }
        if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) { throw '未找到 .NET SDK，请准备 .NET 6 SDK；源码已保留。' }
        $sdks = @(& dotnet --list-sdks)
        if ($LASTEXITCODE -ne 0 -or -not ($sdks -match '^6\.')) { throw '未找到 .NET 6 SDK，请由玩家准备，不自动安装。' }
        $fixed += $manifest.EntryDll
    }
    $resources = @()
    $targets = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    foreach ($name in $fixed) { [void]$targets.Add($name) }
    if ($ResourceManifest) {
        $resourcePath = Resolve-SourcePath $ResourceManifest
        $resources = @(Read-JsonFile $resourcePath)
        $raw = Get-Content -LiteralPath $resourcePath -Raw -Encoding UTF8
        if (-not $raw.TrimStart().StartsWith('[')) { throw 'ResourceManifest 必须为 JSON 数组。' }
        if ($resources.Count -gt 1000) { throw '资源清单超过 1000 项。' }
        foreach ($entry in $resources) {
            if ($entry.source -isnot [string] -or $entry.target -isnot [string]) { throw '资源条目需要 source 和 target 字符串。' }
            Assert-Relative $entry.source
            Assert-Relative $entry.target
            if (-not $targets.Add($entry.target)) { throw '资源目标重复或覆盖固定交付文件。' }
            if ($referenceNames -contains [IO.Path]::GetFileName($entry.source) -or $referenceNames -contains [IO.Path]::GetFileName($entry.target)) { throw '不得交付游戏或 SMAPI 参考程序集。' }
            $path = Resolve-SourcePath (Join-Path $modRoot $entry.source)
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "资源文件不存在：$($entry.source)" }
            if ([IO.Path]::GetExtension($path) -eq '.json') { $null = Read-JsonFile $path }
        }
    }
    New-Item -ItemType Directory -Path $output | Out-Null
    $package = Join-Path $output 'package'
    New-Item -ItemType Directory -Path $package | Out-Null
    if ($Kind -eq 'code') {
        $bin = Join-Path $output 'bin'
        & dotnet build $project --configuration Release --output $bin "-p:ReferenceDirectory=$references" "-p:BaseIntermediateOutputPath=$output/obj/"
        if ($LASTEXITCODE -ne 0) { throw 'C# 构建失败，未生成可交付版本。' }
        $dll = Join-Path $bin $manifest.EntryDll
        if (-not (Test-Path -LiteralPath $dll -PathType Leaf)) { throw '构建未产生 manifest.EntryDll 对应的 DLL。' }
        Copy-Item -LiteralPath $dll -Destination (Join-Path $package $manifest.EntryDll)
    }
    Copy-Item -LiteralPath $manifestPath -Destination (Join-Path $package 'manifest.json')
    Copy-Item -LiteralPath $readmePath -Destination (Join-Path $package 'README.md')
    if ($Kind -eq 'content-pack') { Copy-Item -LiteralPath (Join-Path $modRoot 'content.json') -Destination (Join-Path $package 'content.json') }
    foreach ($entry in $resources) {
        $destination = Join-Path $package $entry.target
        $parent = [IO.Path]::GetDirectoryName($destination)
        if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
        Copy-Item -LiteralPath (Join-Path $modRoot $entry.source) -Destination $destination
    }
    Write-Output "已完成 $Kind 校验与整理：$package。请通过 prepare_delivery 登记；游戏效果仍待玩家验证。"
    exit 0
} catch { Write-Error $_ -ErrorAction Continue; exit 1 }
