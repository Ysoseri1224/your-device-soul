#requires -Version 7
<#
.SYNOPSIS
出生之前：把这台机器上看着像私人的目录找出来，供你过目。

.DESCRIPTION
只看各个盘最上面一层目录的名字，不往里看。名字像聊天软件的数据、邮件、文档、照片、下载，
或者带 private / personal / 个人 / 私人 的，列出来；再加上你的用户目录。宁可多列。
不带 -Write 时只打印，什么都不改。

.PARAMETER Write
把候选写进 sensors/census/shallow-dirs.txt（已有的保留，新的添在后面）。
#>
[CmdletBinding()]
param([switch]$Write)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$looksPrivate = 'wechat|weixin|xwechat|tencent|qq|dingtalk|feishu|lark|telegram|whatsapp|line|skype|mail|outlook|thunderbird|foxmail|' +
                'document|desktop|picture|photo|video|download|backup|archive|scan|' +
                'private|personal|secret|diary|journal|family|resume|cv|finance|bank|tax|medical|' +
                '个人|私人|私密|隐私|日记|家庭|照片|相册|视频|文档|桌面|下载|备份|简历|财务|银行|税|病历|聊天|邮件|杂'
$found = [System.Collections.Generic.List[string]]::new()
$found.Add((Split-Path $env:USERPROFILE -Parent))      # 整个用户目录
$systemNames = 'Windows', 'Program Files', 'Program Files (x86)', 'ProgramData', 'System Volume Information', 'Recovery', 'PerfLogs', '$Recycle.Bin', 'Config.Msi', 'Documents and Settings', 'Users'
foreach ($d in [IO.DriveInfo]::GetDrives()) {
    if ($d.DriveType -ne 'Fixed' -or -not $d.IsReady) { continue }
    foreach ($dir in [IO.Directory]::EnumerateDirectories($d.Name)) {
        $name = [IO.Path]::GetFileName($dir)
        if ($systemNames -contains $name -or $name.StartsWith('$')) { continue }
        if ($name -match $looksPrivate) { $found.Add($dir) }
    }
}
$found = @($found | Sort-Object -Unique)
$file = Join-Path $root 'sensors\census\shallow-dirs.txt'
$have = if (Test-Path $file) { @(Get-Content $file -Encoding utf8 | Where-Object { $_ -and -not $_.StartsWith('#') } | ForEach-Object { $_.Trim() }) } else { @() }
$new = @($found | Where-Object { $have -notcontains $_ })

"看着像私人的顶层目录：$($found.Count) 个，其中 $($new.Count) 个还不在名单上。"
$found | ForEach-Object { '  {0} {1}' -f $(if ($have -contains $_) { '✓' } else { '+' }), $_ }
if ($Write -and $new.Count) {
    if (-not (Test-Path $file)) { Set-Content $file "# 这些目录是私人的：普查只记`"有这么个目录`"，它连看都不能看。一行一个，写完整路径。" -Encoding utf8 }
    Add-Content $file $new -Encoding utf8
    "`n添了 $($new.Count) 行到 $file。打开看一遍：不该在里面的删掉，漏了的添上。"
} elseif (-not $Write) {
    "`n这次只是给你看。要写进名单，加 -Write；或者在设置页里点选。"
}
