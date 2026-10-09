param([Parameter(Mandatory=$true)][string]$ReferenceDirectory)
$ErrorActionPreference = 'Stop'
try {
    if (-not (Test-Path -LiteralPath $ReferenceDirectory -PathType Container)) { throw '参考程序集目录不存在。' }
    $required = @('UnityEngine.CoreModule.dll', 'UnityEngine.IMGUIModule.dll')
    $missing = @($required | Where-Object { -not (Test-Path -LiteralPath (Join-Path $ReferenceDirectory $_) -PathType Leaf) })
    $gameAssemblies = @(Get-ChildItem -LiteralPath $ReferenceDirectory -Filter 'TeamSoda.*.dll' -File)
    if ($gameAssemblies.Count -eq 0) { $missing += 'TeamSoda.*.dll（需包含 Duckov.Modding.ModBehaviour）' }
    if ($missing.Count -gt 0) { throw ('缺少参考程序集：' + ($missing -join ', ')) }
    if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) { throw '未找到 .NET SDK 的 dotnet 命令。源码已保留。' }
    Write-Output '参考程序集与 dotnet 命令已找到；不代表游戏运行验证通过。'
    exit 0
} catch { Write-Error $_ -ErrorAction Continue; exit 1 }
