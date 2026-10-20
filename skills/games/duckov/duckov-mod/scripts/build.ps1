param(
    [Parameter(Mandatory=$true)][string]$ProjectPath,
    [Parameter(Mandatory=$true)][string]$ReferenceDirectory,
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
try {
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'prepare.ps1') -ReferenceDirectory $ReferenceDirectory
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $sourceRoot = [IO.Path]::GetFullPath((Get-Location).Path).TrimEnd('\') + '\'
    $output = [IO.Path]::GetFullPath($OutputDirectory)
    if (-not $output.StartsWith($sourceRoot, [StringComparison]::OrdinalIgnoreCase)) { throw '构建输出必须位于当前 source 目录内。' }
    if (Test-Path -LiteralPath $output) { throw '本次构建输出目录已存在，请使用当前 taskId 的全新目录。' }
    if (-not (Test-Path -LiteralPath $ProjectPath -PathType Leaf)) { throw 'C# 工程不存在。' }
    New-Item -ItemType Directory -Path $output | Out-Null
    & dotnet build $ProjectPath --configuration Release --output $output "-p:ReferenceDirectory=$ReferenceDirectory" "-p:BaseIntermediateOutputPath=$output/obj/"
    exit $LASTEXITCODE
} catch { Write-Error $_ -ErrorAction Continue; exit 1 }
