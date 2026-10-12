# 第一遍普查没查的三样：非系统盘上各个顶层目录占了多大、安全软件（火绒）的样子、系统日志里最近谁在报错。
# 都只读、标准权限。读不到的就记"读不了"，不绕。

# 各盘顶层目录的大小。只对非系统盘做（系统盘的 Windows、Program Files 算一遍要很久，也没什么可看的）。
# 大小天天在变，所以取整到 GB：不到 1 GB 的记 "<1"。这样两次普查之间只有真的涨了一截才会显出差别。
# 私人的目录也只是一个总数，里面有什么不看。
Register-Collector -Name 'dirsizes' -File 'dirsizes.tsv' -Header "dir`tsize_gb`tfiles" -Script {
    $opt = [IO.EnumerationOptions]::new()
    $opt.IgnoreInaccessible = $true
    $opt.RecurseSubdirectories = $true
    $opt.AttributesToSkip = [IO.FileAttributes]::ReparsePoint
    $top = [IO.EnumerationOptions]::new(); $top.IgnoreInaccessible = $true; $top.AttributesToSkip = [IO.FileAttributes]::ReparsePoint
    $system = [IO.Path]::GetPathRoot($env:SystemRoot)
    foreach ($d in [IO.DriveInfo]::GetDrives()) {
        if ($d.DriveType -ne 'Fixed' -or -not $d.IsReady -or $d.Name -eq $system) { continue }
        foreach ($dir in [IO.Directory]::EnumerateDirectories($d.Name, '*', $top)) {
            if ($script:SkipTopDirs -contains [IO.Path]::GetFileName($dir)) { continue }
            [long]$bytes = 0; $n = 0
            try { foreach ($f in [IO.DirectoryInfo]::new($dir).EnumerateFiles('*', $opt)) { $bytes += $f.Length; $n++ } } catch { }
            $gb = if ($bytes -lt 1GB) { '<1' } else { '{0:N0}' -f ($bytes / 1GB) }
            # 文件数取两位有效数字，同样是为了不让每天的小变动都显出来
            $round = if ($n -lt 100) { $n } else { $p = [Math]::Pow(10, [Math]::Floor([Math]::Log10($n)) - 1); [long]([Math]::Round($n / $p) * $p) }
            Join-Row $dir $gb $round
        }
    }
}

# 火绒：装在哪、什么版本、几个服务各是什么状态。它的设置文件受它自己保护，读不了就照记。
Register-Collector -Name 'security' -File 'security.tsv' -Header "item`tvalue" -Script {
    $svc = @(Get-Service -ErrorAction SilentlyContinue | Where-Object { $_.Name -match '^(HipsDaemon|hrdevmon|sysdiag|HRWSCCtrl)' })
    if (-not $svc.Count) { Join-Row '火绒' '没有装，或者服务名变了'; return }
    foreach ($s in $svc) { Join-Row "服务 $($s.Name)" "$($s.Status)，$($s.StartType)" }
    $dir = (Get-CimInstance Win32_Service -Filter "Name='HipsDaemon'" -ErrorAction SilentlyContinue).PathName -replace '"', '' | Split-Path -ErrorAction SilentlyContinue
    if ($dir) {
        Join-Row '装在' $dir
        $exe = Join-Path $dir 'HipsMain.exe'
        if (Test-Path -LiteralPath $exe) { Join-Row '版本' ((Get-Item -LiteralPath $exe).VersionInfo.ProductVersion) }
    }
    # 它的设置和日志在 ProgramData 下，受自我保护。只看能不能列出来，不去读内容
    $data = Join-Path $env:ProgramData 'Huorong\Sysdiag'
    if (Test-Path -LiteralPath $data) {
        try { $names = @(Get-ChildItem -LiteralPath $data -Name -ErrorAction Stop); Join-Row '设置目录' "$data（列得出 $($names.Count) 项，内容没有读）" }
        catch { Join-Row '设置目录' "$data（读不了：被它自己保护着）" }
    }
    Join-Row 'Windows 安全中心登记的杀毒软件' ((Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct -ErrorAction SilentlyContinue | ForEach-Object displayName | Sort-Object -Unique) -join '、')
}

# 系统日志：最近七天，"系统"和"应用程序"两本日志里，谁报了多少次错。只数数，不抄日志的正文（正文里会有路径和用户名）。
Register-Collector -Name 'events' -File 'events.tsv' -Header "log`tprovider`tevent_id`tlevel`tcount_7d" -Script {
    $since = (Get-Date).AddDays(-7)
    foreach ($log in 'System', 'Application') {
        $ev = @(Get-WinEvent -FilterHashtable @{ LogName = $log; Level = 1, 2; StartTime = $since } -ErrorAction SilentlyContinue)
        $ev | Group-Object ProviderName, Id, LevelDisplayName | Sort-Object Count -Descending | Select-Object -First 25 | ForEach-Object {
            $e = $_.Group[0]
            # 次数也取个整，免得每天都变：1–4 照记，往上记成 5+、10+、50+、100+
            $c = $_.Count; $bucket = if ($c -lt 5) { "$c" } elseif ($c -lt 10) { '5+' } elseif ($c -lt 50) { '10+' } elseif ($c -lt 100) { '50+' } else { '100+' }
            Join-Row $log $e.ProviderName $e.Id $e.LevelDisplayName $bucket
        }
    }
}
