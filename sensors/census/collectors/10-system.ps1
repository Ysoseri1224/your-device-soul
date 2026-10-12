# 机器本身：系统、硬件。保持书写顺序，不排序。
Register-Collector -Name 'system' -File 'system.txt' -Sort $false -Script {
    $os = Get-CimInstance Win32_OperatingSystem
    $cs = Get-CimInstance Win32_ComputerSystem
    $lines = [Collections.Generic.List[string]]::new()
    $lines.Add((Join-Row 'os' "$($os.Caption) $($os.Version)"))
    $lines.Add((Join-Row 'arch' $os.OSArchitecture))
    $lines.Add((Join-Row 'model' "$($cs.Manufacturer) $($cs.Model)"))
    foreach ($cpu in @(Get-CimInstance Win32_Processor)) {
        $lines.Add((Join-Row 'cpu' $cpu.Name "$($cpu.NumberOfCores) cores / $($cpu.NumberOfLogicalProcessors) threads"))
    }
    $lines.Add((Join-Row 'ram_gb' ('{0:N1}' -f ($cs.TotalPhysicalMemory / 1GB))))
    foreach ($g in @(Get-CimInstance Win32_VideoController | Sort-Object Name)) { $lines.Add((Join-Row 'gpu' $g.Name)) }
    foreach ($d in @(Get-CimInstance Win32_DiskDrive | Sort-Object Index)) {
        $lines.Add((Join-Row 'disk' $d.Model ('{0:N0} GB' -f ($d.Size / 1GB)) $d.MediaType))
    }
    $bat = @(Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue)
    $lines.Add((Join-Row 'battery' $(if ($bat.Count) { 'present' } else { 'none' })))
    foreach ($m in @(Get-CimInstance Win32_DesktopMonitor -ErrorAction SilentlyContinue | Sort-Object Name)) {
        $lines.Add((Join-Row 'monitor' $m.Name "$($m.ScreenWidth)x$($m.ScreenHeight)"))
    }
    $lines.Add((Join-Row 'timezone' ([TimeZoneInfo]::Local.Id)))
    $lines.Add((Join-Row 'ui_language' ([cultureinfo]::CurrentUICulture.Name)))
    $lines
}
