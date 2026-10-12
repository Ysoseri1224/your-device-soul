#requires -Version 7
<#
.SYNOPSIS
推之前的审查：在要出这台机器的文件里找个人信息和密钥。

.DESCRIPTION
找：邮箱地址、账号编号的样子、像密钥的字符串、这台机器的 Windows 用户名和机器名、私人名单里的每一个目录名，
以及你自己写在 local/audit-words.txt 里的词（真名、学校、单位，一行一个；这个文件不出这台机器）。
有命中就打出文件和行号，命中的内容本身遮掉，并以失败退出。

.PARAMETER Path
查哪里。不写就查 git 会推出去的所有文件（已跟踪的加上没被忽略的新文件）。

.PARAMETER Allow
这些词不算（比如你公开的 GitHub 用户名）。
#>
[CmdletBinding()]
param([string]$Path, [string[]]$Allow = @())
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $root

$files = if ($Path) { Get-ChildItem -LiteralPath $Path -Recurse -File | ForEach-Object FullName }
         else { git ls-files --cached --others --exclude-standard | ForEach-Object { Join-Path $root $_ } }
$skip = '\.(png|jpg|jpeg|gif|ico|icns|woff2?|ttf|lock|exe|dll|pdb|zip)$'

# 固定的几种样子
$shapes = [ordered]@{
    '邮箱地址'   = '[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+\.[A-Za-z]{2,}'
    '账号编号'   = '\bwxid_[A-Za-z0-9]{6,}\b'
    '像密钥的串' = '\b(sk|rk|pk|ghp|gho|github_pat|xox[abp])[-_][A-Za-z0-9_-]{16,}\b'
    '长串数字'   = '(?<![\d.])\d{11,}(?![\d.])'
}
# 这台机器特有的词
$words = [System.Collections.Generic.List[string]]::new()
foreach ($w in $env:USERNAME, $env:COMPUTERNAME) { if ($w -and $w.Length -ge 3) { $words.Add($w) } }
$private = Join-Path $root 'sensors\census\shallow-dirs.txt'
if (Test-Path $private) {
    Get-Content $private -Encoding utf8 | Where-Object { $_ -and -not $_.StartsWith('#') } | ForEach-Object {
        $leaf = Split-Path $_.Trim() -Leaf
        # 通用的目录名（Users、Documents 这类）不算个人信息
        if ($leaf.Length -ge 2 -and $leaf -notmatch '^(Users|Documents|Downloads|Desktop|Pictures|Archive|UserData|Tencent|WeChat|Weixin|QQNT|personal|private|xwechat_files|WeChat Files|MailMasterData|JianyingPro Drafts|QQdowmload)$') { $words.Add($leaf) }
    }
}
$mine = Join-Path $root 'local\audit-words.txt'
if (Test-Path $mine) { Get-Content $mine -Encoding utf8 | Where-Object { $_.Trim().Length -ge 2 } | ForEach-Object { $words.Add($_.Trim()) } }
$words = @($words | Sort-Object -Unique | Where-Object { $Allow -notcontains $_ })
# 明知不算的：示例里的占位邮箱、提交说明里的机器人地址
$fine = 'noreply@|@example\.|user@|name@|you@|xxx@|\.png@|@2x|@types|@media|@keyframes|@import|@font-face|@oai|@anthropic-ai|\$\w+@|git@github\.com'

$hits = 0
foreach ($f in $files) {
    if ($f -match $skip -or -not (Test-Path -LiteralPath $f)) { continue }
    if ((Get-Item -LiteralPath $f).Length -gt 3MB) { continue }
    $rel = [IO.Path]::GetRelativePath($root, $f)
    if ($rel -like 'tools\audit.ps1') { continue }
    $n = 0
    foreach ($line in [IO.File]::ReadLines($f)) {
        $n++
        foreach ($k in $shapes.Keys) {
            foreach ($m in [regex]::Matches($line, $shapes[$k])) {
                if ($k -eq '邮箱地址' -and $line -match $fine) { continue }
                if ($k -eq '长串数字' -and $line -match 'at"|"at"|unix|timestamp|\d{13,}') { continue }      # 时间戳
                $v = $m.Value
                if ($k -eq '像密钥的串' -and $v -match 'abcdef|123456|xxxx|example|test|fake') { continue }      # 测试里编的假密钥
                $shown = $v.Substring(0, [Math]::Min(3, $v.Length)) + '…'
                '{0}:{1}  {2}  {3}' -f $rel, $n, $k, $shown; $hits++
            }
        }
        foreach ($w in $words) {
            if ($line.IndexOf($w, [StringComparison]::OrdinalIgnoreCase) -ge 0) { '{0}:{1}  这台机器特有的词  {2}…' -f $rel, $n, $w.Substring(0, [Math]::Min(2, $w.Length)); $hits++ }
        }
    }
}
if ($hits) { Write-Host "`n$hits 处要看。逐个确认：是真的个人信息就改掉；不是，就把那个词加进 -Allow。" -ForegroundColor Yellow; exit 1 }
Write-Host "查了 $(@($files).Count) 个文件，没有发现要看的。" -ForegroundColor Green
exit 0
