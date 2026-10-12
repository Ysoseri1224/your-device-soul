# 装了什么：传统安装程序登记的，和应用商店装的。
Register-Collector -Name 'software' -File 'software.tsv' -Header "name`tversion`tpublisher`tscope`tinstall_location" -Script {
    $roots = [ordered]@{
        'machine'    = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
        'machine-32' = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
        'user'       = 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
    }
    foreach ($scope in $roots.Keys) {
        foreach ($e in @(Get-ItemProperty -Path $roots[$scope] -ErrorAction SilentlyContinue)) {
            $p = $e.PSObject.Properties
            $name = if ($p['DisplayName']) { [string]$e.DisplayName } else { '' }
            if ([string]::IsNullOrWhiteSpace($name)) { continue }
            if ($p['SystemComponent'] -and $e.SystemComponent -eq 1) { continue }
            $ver = if ($p['DisplayVersion']) { $e.DisplayVersion } else { '' }
            $pub = if ($p['Publisher']) { $e.Publisher } else { '' }
            $loc = if ($p['InstallLocation']) { $e.InstallLocation } else { '' }
            Join-Row $name $ver $pub $scope $loc
        }
    }
}

Register-Collector -Name 'store-apps' -File 'store-apps.tsv' -Header "name`tversion`tpublisher_id" -Script {
    # 应用商店的清单在 PowerShell 7 里取不到，所以借系统自带的旧版 PowerShell 来问。
    $ask = 'powershell.exe -NoProfile -Command "Get-AppxPackage | Where-Object { -not $_.IsFramework } | ForEach-Object { $_.Name + [char]9 + $_.Version + [char]9 + $_.PublisherId }"'
    $out = Invoke-Tool $ask 40000
    if ($out) {
        foreach ($l in ($out -split "`r?`n")) {
            $f = $l -split "`t"
            if ($f.Count -ge 2) { Join-Row $f[0] $f[1] $(if ($f.Count -ge 3) { $f[2] } else { '' }) }
        }
    }
}
