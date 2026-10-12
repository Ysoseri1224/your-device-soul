#requires -Version 7
<#
.SYNOPSIS
提权体检：普查里需要管理员权限才看得全的那部分。

.DESCRIPTION
全部只读，不改任何东西。查的内容见 collectors-admin 目录：
打开了哪些 Windows 功能、本机账户、安全状态、各盘剩余空间、硬盘和电池的健康、
最近的系统错误、C 盘各目录的大小。

没有管理员权限时，它会自己请求一次。系统会弹出确认框，由机主点"是"。
#>
[CmdletBinding()]
param(
    [string]$OutDir = (Join-Path $PSScriptRoot '..\..\memory\body\admin'),
    # 只跑其中几项，逗号分隔
    [string]$Only = ''
)
$OutDir = [IO.Path]::GetFullPath($OutDir)
$me = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
$elevated = $me.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $elevated) {
    '这份脚本需要管理员权限。马上会弹出系统的确认框，请点"是"。它只读不改。'
    $argList = @('-NoProfile', '-File', "`"$PSCommandPath`"", '-OutDir', "`"$OutDir`"")
    if ($Only) { $argList += @('-Only', $Only) }
    try {
        $p = Start-Process -FilePath 'pwsh' -Verb RunAs -ArgumentList $argList -Wait -PassThru -ErrorAction Stop
    } catch {
        "没有拿到授权：$($_.Exception.Message)"
        exit 2
    }
    "提权体检结束，退出码 $($p.ExitCode)。结果在 $OutDir"
    exit $p.ExitCode
}

if ($Only) { & (Join-Path $PSScriptRoot 'census.ps1') -OutDir $OutDir -Set admin -Only $Only }
else { & (Join-Path $PSScriptRoot 'census.ps1') -OutDir $OutDir -Set admin }
$code = $LASTEXITCODE
'三秒后关闭这个窗口。'
Start-Sleep -Seconds 3
exit $code
