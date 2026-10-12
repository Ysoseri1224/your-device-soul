#requires -Version 7
<#
.SYNOPSIS
普查：把这台机器“表面”一层列成纯文本快照。

.DESCRIPTION
只读，标准权限，不调模型。每个采集器写一个文件。同样的机器状态写出同样的文件，
所以把输出目录交给 git，今天和昨天一比就是“我身上变了什么”。

.PARAMETER OutDir
快照写到哪。默认是 local/census（不进仓库），调试用。正式普查时指向 memory/body。

.PARAMETER Only
只跑指定的采集器，例如 -Only services,tasks。

.PARAMETER RepoDepth
找仓库时从盘的根目录往下看几层。
#>
[CmdletBinding()]
param(
    [string]$OutDir = (Join-Path $PSScriptRoot '..\..\local\census'),
    [string[]]$Only,
    [int]$RepoDepth = 5,
    # standard 是平时的普查；admin 是需要管理员权限的那几项，由 census-admin.ps1 调用
    [ValidateSet('standard', 'admin')][string]$Set = 'standard'
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$PSStyle.OutputRendering = 'PlainText'

. (Join-Path $PSScriptRoot 'lib.ps1')
$script:RepoDepth = $RepoDepth
$collectorDir = if ($Set -eq 'admin') { 'collectors-admin' } else { 'collectors' }
Get-ChildItem (Join-Path $PSScriptRoot $collectorDir) -Filter *.ps1 | Sort-Object Name | ForEach-Object { . $_.FullName }

$OutDir = [IO.Path]::GetFullPath($OutDir)
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$script:OutDir = $OutDir

$names = @($script:Collectors.Keys)
if ($Only) {
    # 用 pwsh -File 调用时，"a,b,c" 会作为一整个字符串传进来，这里拆开
    $Only = @($Only | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    $unknown = @($Only | Where-Object { $names -notcontains $_ })
    if ($unknown.Count) { throw "没有这个采集器：$($unknown -join ', ')。可用的有：$($names -join ', ')" }
    $names = @($names | Where-Object { $Only -contains $_ })
}

$errors = [Collections.Generic.List[string]]::new()
$report = foreach ($name in $names) {
    $c = $script:Collectors[$name]
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $count = 0
    $status = 'ok'
    try {
        $lines = @(& $c.Script | ForEach-Object { [string]$_ })
        $count = Write-Snapshot -Path (Join-Path $OutDir $c.File) -Lines $lines -Header $c.Header -Sort $c.Sort
    } catch {
        $status = 'failed'
        $errors.Add("$name`t$($_.Exception.Message)")
    }
    [pscustomobject]@{ collector = $name; file = $c.File; lines = $count; seconds = [math]::Round($sw.Elapsed.TotalSeconds, 1); status = $status }
}

$errFile = Join-Path $OutDir '_errors.txt'
if ($errors.Count) { Write-Snapshot -Path $errFile -Lines $errors.ToArray() -Sort $false | Out-Null }
elseif (Test-Path $errFile) { Remove-Item $errFile }

$report | Format-Table -AutoSize
"写到了 $OutDir"
if ($errors.Count) { "有 $($errors.Count) 个采集器失败，见 _errors.txt"; exit 1 }
