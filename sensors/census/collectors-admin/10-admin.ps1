# 需要管理员权限才看得全的部分。全部只读，不改任何东西。
# 文件名以 live- 开头的是天天在变的量，不进仓库。

Register-Collector -Name 'features' -File 'features.tsv' -Header "feature" -Script {
    # 打开了哪些 Windows 可选功能（Hyper-V、WSL、沙盒这些）
    foreach ($f in @(Get-WindowsOptionalFeature -Online -ErrorAction Stop | Where-Object { $_.State -eq 'Enabled' })) {
        Join-Row $f.FeatureName
    }
}

Register-Collector -Name 'local-users' -File 'local-users.tsv' -Header "kind`tname`tdetail" -Script {
    # 这台机器上有哪些账户，哪些是管理员，每个用户目录属于谁
    foreach ($u in @(Get-LocalUser)) {
        Join-Row 'account' $u.Name "enabled=$($u.Enabled); $($u.Description)"
    }
    foreach ($m in @(Get-LocalGroupMember -SID 'S-1-5-32-544' -ErrorAction SilentlyContinue)) {
        Join-Row 'administrator' $m.Name $m.ObjectClass
    }
    $lm = [Microsoft.Win32.Registry]::LocalMachine
    $list = 'SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList'
    foreach ($sid in (Get-RegSubKeyNames $lm $list)) {
        if ($sid -notlike 'S-1-5-21-*') { continue }
        $path = Get-RegValue $lm "$list\$sid" 'ProfileImagePath'
        if (-not $path) { continue }
        $owner = try { ([Security.Principal.SecurityIdentifier]$sid).Translate([Security.Principal.NTAccount]).Value } catch { '(账户已不存在)' }
        Join-Row 'profile-folder' $path $owner
    }
}

Register-Collector -Name 'security' -File 'security.tsv' -Header "item`tvalue" -Script {
    # 机器的安全状态：安全启动、TPM、磁盘加密、提权确认、防病毒、防火墙
    Join-Row 'secure-boot' $(try { [string](Confirm-SecureBootUEFI -ErrorAction Stop) } catch { 'unknown' })
    try { $t = Get-Tpm -ErrorAction Stop; Join-Row 'tpm' "present=$($t.TpmPresent); ready=$($t.TpmReady)" } catch { Join-Row 'tpm' 'unknown' }
    try {
        foreach ($v in @(Get-BitLockerVolume -ErrorAction Stop)) { Join-Row "disk-encryption $($v.MountPoint)" "$($v.VolumeStatus); protection=$($v.ProtectionStatus)" }
    } catch { Join-Row 'disk-encryption' 'unknown' }
    $lm = [Microsoft.Win32.Registry]::LocalMachine
    $pol = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System'
    Join-Row 'uac-enabled' (Get-RegValue $lm $pol 'EnableLUA')
    Join-Row 'uac-admin-prompt-level' (Get-RegValue $lm $pol 'ConsentPromptBehaviorAdmin')
    try { $d = Get-MpComputerStatus -ErrorAction Stop; Join-Row 'defender' "antivirus=$($d.AntivirusEnabled); realtime=$($d.RealTimeProtectionEnabled)" } catch { Join-Row 'defender' 'unknown' }
    foreach ($a in @(Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct -ErrorAction SilentlyContinue)) {
        Join-Row 'antivirus-registered' $a.displayName
    }
    foreach ($p in @(Get-NetFirewallProfile -ErrorAction SilentlyContinue)) { Join-Row "firewall $($p.Name)" "enabled=$($p.Enabled)" }
}

Register-Collector -Name 'all-tasks' -File 'all-tasks-count.tsv' -Header "scope`tcount" -Script {
    # 管理员能看到的计划任务比普通权限多，记个总数做对照
    $all = @(Get-ScheduledTask -ErrorAction SilentlyContinue)
    Join-Row 'total' $all.Count
    Join-Row 'outside-microsoft' @($all | Where-Object { $_.TaskPath -notlike '\Microsoft\*' }).Count
}

Register-Collector -Name 'live-health' -File 'live-health.tsv' -Sort $false -Header "kind`tname`tvalue" -Script {
    # 身体此刻的状况：各盘还剩多少、硬盘和电池的健康、最近的系统错误
    foreach ($d in [IO.DriveInfo]::GetDrives()) {
        if ($d.DriveType -ne 'Fixed' -or -not $d.IsReady) { continue }
        $pct = [math]::Round(100 * $d.AvailableFreeSpace / $d.TotalSize)
        Join-Row 'free-space' $d.Name ('{0:N1} GB free of {1:N0} GB ({2}%)' -f ($d.AvailableFreeSpace / 1GB), ($d.TotalSize / 1GB), $pct)
    }
    foreach ($f in 'pagefile.sys', 'hiberfil.sys', 'swapfile.sys') {
        $i = Get-Item -LiteralPath "C:\$f" -Force -ErrorAction SilentlyContinue
        if ($i) { Join-Row 'system-file' $f ('{0:N1} GB' -f ($i.Length / 1GB)) }
    }
    foreach ($pd in @(Get-PhysicalDisk -ErrorAction SilentlyContinue)) {
        Join-Row 'disk' $pd.FriendlyName "health=$($pd.HealthStatus); status=$($pd.OperationalStatus); type=$($pd.MediaType)"
        $r = $pd | Get-StorageReliabilityCounter -ErrorAction SilentlyContinue
        if ($r) {
            Join-Row 'disk-wear' $pd.FriendlyName "wear=$($r.Wear)%; temperature=$($r.Temperature)C; power-on-hours=$($r.PowerOnHours); read-errors=$($r.ReadErrorsTotal); write-errors=$($r.WriteErrorsTotal)"
        }
    }
    # 不同厂商的电池驱动提供的字段不一样，取不到就算了
    $pick = {
        param($class, $prop)
        $o = Get-CimInstance -Namespace root/wmi -ClassName $class -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($o -and $o.PSObject.Properties[$prop]) { $o.$prop } else { $null }
    }
    $design = & $pick 'BatteryStaticData' 'DesignedCapacity'
    $full = & $pick 'BatteryFullChargedCapacity' 'FullChargedCapacity'
    $cycles = & $pick 'BatteryCycleCount' 'CycleCount'
    if (-not $design) {
        # 退一步：让系统出一份电池报告，从里面读设计容量
        $xml = Join-Path $env:TEMP 'ysoul-battery.xml'
        & powercfg.exe /batteryreport /xml /output $xml 2>$null | Out-Null
        if (Test-Path $xml) {
            try {
                $b = ([xml](Get-Content $xml -Raw)).BatteryReport.Batteries.Battery | Select-Object -First 1
                $design = [int64]$b.DesignCapacity; $full = [int64]$b.FullChargeCapacity; $cycles = $b.CycleCount
            } catch { }
            Remove-Item $xml -ErrorAction SilentlyContinue
        }
    }
    if ($design -and $full) {
        Join-Row 'battery' 'capacity' ('{0} of {1} mWh ({2}% of design); cycles={3}' -f $full, $design, [math]::Round(100 * $full / $design), $cycles)
    }
    $os = Get-CimInstance Win32_OperatingSystem
    Join-Row 'uptime' 'last-boot' $os.LastBootUpTime.ToString('yyyy-MM-dd HH:mm')
    Join-Row 'memory' 'in-use' ('{0:N1} of {1:N1} GB' -f (($os.TotalVisibleMemorySize - $os.FreePhysicalMemory) / 1MB), ($os.TotalVisibleMemorySize / 1MB))
    try {
        $ev = @(Get-WinEvent -FilterHashtable @{ LogName = 'System'; Level = 1, 2; StartTime = (Get-Date).AddDays(-30) } -ErrorAction Stop)
        Join-Row 'system-errors' 'last-30-days' $ev.Count
        foreach ($g in ($ev | Group-Object ProviderName | Sort-Object Count -Descending | Select-Object -First 10)) {
            Join-Row 'system-errors-by-source' $g.Name $g.Count
        }
    } catch { Join-Row 'system-errors' 'last-30-days' 'none or unreadable' }
}

function Get-DirBytes {
    # 借 robocopy 的"只列不拷"来数一个目录有多大。它的汇总里第三行是字节数，不依赖界面语言。
    param([string]$Path)
    $out = & robocopy.exe $Path 'C:\__ysoul_nowhere__' /L /S /NJH /BYTES /NFL /NDL /XJ /R:0 /W:0 2>$null
    $sums = @($out | Where-Object { $_ -match ':\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s*$' })
    if ($sums.Count -ge 3 -and $sums[2] -match ':\s+(\d+)') { return [int64]$Matches[1] }
    return -1
}

Register-Collector -Name 'live-sizes' -File 'live-sizes.tsv' -Sort $false -Header "where`tsize_gb" -Script {
    # C 盘是系统盘，最容易满。数一数它最上面一层和每个用户目录各有多大。
    $rows = [Collections.Generic.List[object]]::new()
    $targets = @(Get-ChildItem -LiteralPath 'C:\' -Directory -Force -ErrorAction SilentlyContinue | Where-Object { -not $_.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint) -and $_.Name -notin '$Recycle.Bin', 'System Volume Information', 'Users' })
    $targets += @(Get-ChildItem -LiteralPath 'C:\Users' -Directory -Force -ErrorAction SilentlyContinue | Where-Object { -not $_.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint) })
    foreach ($t in $targets) {
        Write-Host "  数大小：$($t.FullName)"
        $rows.Add([pscustomobject]@{ Path = $t.FullName; Bytes = (Get-DirBytes $t.FullName) })
    }
    foreach ($r in ($rows | Sort-Object Bytes -Descending)) {
        Join-Row $r.Path $(if ($r.Bytes -ge 0) { '{0:N2}' -f ($r.Bytes / 1GB) } else { '?' })
    }
}

Register-Collector -Name 'all-tasks-list' -File 'all-tasks.tsv' -Header "task`tenabled`tauthor`trun_as`taction" -Script {
    # 管理员能看到的全部计划任务。和标准权限下的 tasks.tsv 一比，就知道哪些是普通权限看不到的。
    foreach ($t in @(Get-ScheduledTask -ErrorAction SilentlyContinue)) {
        $enabled = if ($t.State -eq 'Disabled') { 'no' } else { 'yes' }
        $actions = @(foreach ($a in @($t.Actions)) {
                $p = $a.PSObject.Properties
                if ($p['Execute']) { ("$($a.Execute) $($a.Arguments)").Trim() }
                elseif ($p['ClassId']) { "com:$($a.ClassId)" }
            }) -join ' | '
        Join-Row "$($t.TaskPath)$($t.TaskName)" $enabled $t.Author $t.Principal.UserId $actions
    }
}
