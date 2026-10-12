# 磁盘：有哪些盘；每个盘最上面两层有哪些目录。剩余空间天天变，不写进快照。
$script:SkipTopDirs = @('$Recycle.Bin', 'System Volume Information', '$WinREAgent', 'Recovery', 'Config.Msi', 'Documents and Settings')

Register-Collector -Name 'volumes' -File 'volumes.tsv' -Header "drive`tlabel`tfilesystem`tsize_gb" -Script {
    foreach ($d in [IO.DriveInfo]::GetDrives()) {
        if ($d.DriveType -ne 'Fixed' -or -not $d.IsReady) { continue }
        Join-Row $d.Name $d.VolumeLabel $d.DriveFormat ('{0:N0}' -f ($d.TotalSize / 1GB))
    }
}

# 只记到第一层的目录，一行一个。等"家的地图"做出来，这份清单并进去，由机主说了算。
$script:ShallowDirs = @()
$shallowFile = Join-Path $PSScriptRoot '..\shallow-dirs.txt'
if (Test-Path -LiteralPath $shallowFile) { $script:ShallowDirs = @(Get-Content -LiteralPath $shallowFile -Encoding utf8 | Where-Object { $_ -and -not $_.StartsWith('#') } | ForEach-Object { $_.Trim() }) }

Register-Collector -Name 'dirs' -File 'dirs.txt' -Script {
    $opt = [IO.EnumerationOptions]::new()
    $opt.IgnoreInaccessible = $true
    $opt.AttributesToSkip = [IO.FileAttributes]::ReparsePoint
    foreach ($d in [IO.DriveInfo]::GetDrives()) {
        if ($d.DriveType -ne 'Fixed' -or -not $d.IsReady) { continue }
        foreach ($top in [IO.Directory]::EnumerateDirectories($d.Name, '*', $opt)) {
            $name = [IO.Path]::GetFileName($top)
            if ($script:SkipTopDirs -contains $name) { continue }
            $top
            # Windows 目录的第二层没有看头；私人的目录只记到第一层，里面有什么不看。其余都往下看一层
            if ($name -eq 'Windows' -or $script:ShallowDirs -contains $top) { continue }
            try {
                foreach ($sub in [IO.Directory]::EnumerateDirectories($top, '*', $opt)) { $sub }
            } catch { }
        }
    }
}
