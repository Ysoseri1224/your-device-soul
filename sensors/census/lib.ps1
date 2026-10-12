# 普查的公共部分：登记采集器、遮掉密钥、写出稳定的纯文本快照。
Set-StrictMode -Version Latest

$script:Collectors = [ordered]@{}

function Register-Collector {
    # 登记一个采集器。Script 返回若干行文本；Sort 为假时保持原顺序。
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$File,
        [Parameter(Mandatory)][scriptblock]$Script,
        [string]$Header = '',
        [bool]$Sort = $true
    )
    $script:Collectors[$Name] = @{ File = $File; Script = $Script; Header = $Header; Sort = $Sort }
}

# 名字像密钥的，或者像账号、姓名、邮箱的，值整个不要。个人信息和密钥同样对待。
$script:SecretNamePattern = '(?i)(key|token|secret|password|passwd|pwd|credential|auth|username|user_name|email|account)'
$script:SecretPatterns = @(
    @{ P = '(?<![A-Za-z0-9])sk-[A-Za-z0-9_\-]{16,}'; R = '<hidden>' }
    @{ P = '(?<![A-Za-z0-9])gh[pousr]_[A-Za-z0-9]{20,}'; R = '<hidden>' }
    @{ P = '(?<![A-Za-z0-9])AKIA[0-9A-Z]{16}'; R = '<hidden>' }
    @{ P = '(?<![A-Za-z0-9])xox[baprs]-[A-Za-z0-9\-]{10,}'; R = '<hidden>' }
    @{ P = 'eyJ[A-Za-z0-9_\-]{20,}\.[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]*'; R = '<hidden>' }
    @{ P = '(://)[^/\s:@]+:[^/\s@]+@'; R = '$1<hidden>@' }
    # 个人信息：邮箱地址、微信的账号编号、整段都是数字的目录名（聊天软件用账号当目录名）
    @{ P = '[A-Za-z0-9._%+\-]+@[A-Za-z0-9\-]+(?:\.[A-Za-z0-9\-]+)+'; R = '<hidden>' }
    @{ P = '(?<![A-Za-z0-9])wxid_[A-Za-z0-9_]+'; R = '<hidden>' }
    @{ P = '(?<=\\)\d{6,}(?=\\|$)'; R = '<hidden>' }
    # 名字后面必须跟着等号或冒号才算。"Password Recovery" 这种只是普通的词。
    @{ P = '(?i)((?:api[_\-]?key|token|secret|password|passwd|pwd)["'']?\s*[=:]\s*["'']?)[^\s"''&;,]{6,}'; R = '$1<hidden>' }
)

function Protect-Secret {
    # 把一段文字里形似密钥的部分换成 <hidden>。
    param([AllowNull()][AllowEmptyString()][string]$Text)
    if ([string]::IsNullOrEmpty($Text)) { return '' }
    $out = $Text
    foreach ($s in $script:SecretPatterns) { $out = [regex]::Replace($out, $s.P, $s.R) }
    return $out
}

function Test-SecretName {
    # 名字本身像密钥（例如环境变量 XXX_TOKEN）时，值整个不要。
    param([string]$Name)
    return [bool]($Name -match $script:SecretNamePattern)
}

function ConvertTo-Field {
    # 一个字段里不能有制表符和换行，否则一行就乱了。
    param([AllowNull()]$Value)
    if ($null -eq $Value) { return '' }
    return (([string]$Value) -replace '[\t\r\n]+', ' ').Trim()
}

function Join-Row {
    # 若干字段拼成一行，用制表符分隔，并遮掉密钥。
    param([Parameter(ValueFromRemainingArguments)][object[]]$Fields)
    $parts = foreach ($f in $Fields) { ConvertTo-Field $f }
    return Protect-Secret ($parts -join "`t")
}

function Write-Snapshot {
    # 写成 UTF-8、LF 换行、去重、按需排序。同样的机器状态写出同样的文件，git 才比得出差异。
    param(
        [Parameter(Mandatory)][string]$Path,
        [AllowEmptyCollection()][string[]]$Lines = @(),
        [string]$Header = '',
        [bool]$Sort = $true
    )
    # 不管采集器自己有没有遮，落盘前统一再遮一遍
    $clean = @($Lines | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { Protect-Secret $_ })
    if ($Sort) { $clean = @($clean | Sort-Object -Unique -Culture ([cultureinfo]::InvariantCulture)) }
    $all = if ($Header) { @($Header) + $clean } else { $clean }
    $dir = Split-Path -Parent $Path
    if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    [IO.File]::WriteAllText($Path, (($all -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))
    return $clean.Count
}

function Invoke-Tool {
    # 运行一个命令行工具并取回输出；超时就放弃，不让普查卡住。
    param(
        [Parameter(Mandatory)][string]$CommandLine,
        [int]$TimeoutMs = 6000,
        [Text.Encoding]$Encoding = [Text.Encoding]::UTF8
    )
    $psi = [Diagnostics.ProcessStartInfo]::new('cmd.exe', "/d /c $CommandLine")
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.StandardOutputEncoding = $Encoding
    $psi.StandardErrorEncoding = $Encoding
    $p = [Diagnostics.Process]::new()
    $p.StartInfo = $psi
    try {
        [void]$p.Start()
        $outTask = $p.StandardOutput.ReadToEndAsync()
        $errTask = $p.StandardError.ReadToEndAsync()
        if (-not $p.WaitForExit($TimeoutMs)) {
            try { $p.Kill($true) } catch { }
            return $null
        }
        $text = $outTask.Result
        if ([string]::IsNullOrWhiteSpace($text)) { $text = $errTask.Result }
        return $text
    } catch {
        return $null
    } finally {
        $p.Dispose()
    }
}

function Protect-ConfigLine {
    # 配置文件里的一行。形如"名字: 值"或"名字 = 值"且名字像密钥时，值整个不要；其余按普通文字遮。
    param([AllowNull()][AllowEmptyString()][string]$Line)
    if ([string]::IsNullOrEmpty($Line)) { return '' }
    $m = [regex]::Match($Line, '^(\s*["'']?)([\w.\-]+)(["'']?\s*[:=]\s*)(.+?)(,?\s*)$')
    if ($m.Success -and (Test-SecretName $m.Groups[2].Value)) {
        return $m.Groups[1].Value + $m.Groups[2].Value + $m.Groups[3].Value + '<hidden>' + $m.Groups[5].Value
    }
    return Protect-Secret $Line
}

function Get-RegSubKeyNames {
    # 列出注册表某个位置下面的子项名字；位置不存在就返回空。
    param([Parameter(Mandatory)][Microsoft.Win32.RegistryKey]$Root, [Parameter(Mandatory)][string]$Path)
    $k = $null
    try { $k = $Root.OpenSubKey($Path) } catch { return }
    if (-not $k) { return }
    try { $k.GetSubKeyNames() } finally { $k.Dispose() }
}

function Get-RegValue {
    # 读注册表里的一个值；读不到返回 $null。Name 为空串表示默认值。
    param([Parameter(Mandatory)][Microsoft.Win32.RegistryKey]$Root, [Parameter(Mandatory)][string]$Path, [AllowEmptyString()][string]$Name = '')
    $k = $null
    try { $k = $Root.OpenSubKey($Path) } catch { return $null }
    if (-not $k) { return $null }
    try { return $k.GetValue($Name) } finally { $k.Dispose() }
}
