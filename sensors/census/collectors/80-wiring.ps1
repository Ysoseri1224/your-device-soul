# 接线一层：这些东西是怎么接进系统的。
# 右键菜单、文件关联、网址协议、程序别名、驱动、网络、字体、电源方案。

Register-Collector -Name 'context-menus' -File 'context-menus.tsv' -Header "where`tkind`tname`ttarget" -Script {
    $cr = [Microsoft.Win32.Registry]::ClassesRoot
    foreach ($where in '*', 'AllFilesystemObjects', 'Directory', 'Directory\Background', 'Folder', 'Drive', 'DesktopBackground') {
        foreach ($n in (Get-RegSubKeyNames $cr "$where\shell")) {
            Join-Row $where 'entry' $n (Get-RegValue $cr "$where\shell\$n\command" '')
        }
        foreach ($n in (Get-RegSubKeyNames $cr "$where\shellex\ContextMenuHandlers")) {
            Join-Row $where 'handler' $n (Get-RegValue $cr "$where\shellex\ContextMenuHandlers\$n" '')
        }
    }
}

Register-Collector -Name 'file-assoc' -File 'file-assoc.tsv' -Header "what`topens_with" -Script {
    # 只记机主自己选过打开方式的那些，系统默认的不记。
    $cu = [Microsoft.Win32.Registry]::CurrentUser
    $exts = 'Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts'
    foreach ($ext in (Get-RegSubKeyNames $cu $exts)) {
        $id = Get-RegValue $cu "$exts\$ext\UserChoice" 'ProgId'
        if ($id) { Join-Row $ext $id }
    }
    $urls = 'Software\Microsoft\Windows\Shell\Associations\UrlAssociations'
    foreach ($scheme in (Get-RegSubKeyNames $cu $urls)) {
        $id = Get-RegValue $cu "$urls\$scheme\UserChoice" 'ProgId'
        if ($id) { Join-Row "${scheme}:" $id }
    }
}

Register-Collector -Name 'url-schemes' -File 'url-schemes.tsv' -Header "scheme`tcommand" -Script {
    # 哪些程序登记了自己的网址协议（例如 vscode:、obsidian:）。
    $cr = [Microsoft.Win32.Registry]::ClassesRoot
    foreach ($n in $cr.GetSubKeyNames()) {
        if ($n.StartsWith('.') -or $n.Contains('\')) { continue }
        $k = $null
        try { $k = $cr.OpenSubKey($n) } catch { continue }
        if (-not $k) { continue }
        try { $isScheme = $null -ne $k.GetValue('URL Protocol') } finally { $k.Dispose() }
        if ($isScheme) { Join-Row $n (Get-RegValue $cr "$n\shell\open\command" '') }
    }
}

Register-Collector -Name 'app-paths' -File 'app-paths.tsv' -Header "scope`tname`tpath" -Script {
    # 在"运行"框里敲名字就能启动的程序。
    $key = 'SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths'
    $roots = [ordered]@{ machine = [Microsoft.Win32.Registry]::LocalMachine; user = [Microsoft.Win32.Registry]::CurrentUser }
    foreach ($scope in $roots.Keys) {
        foreach ($n in (Get-RegSubKeyNames $roots[$scope] $key)) {
            Join-Row $scope $n (Get-RegValue $roots[$scope] "$key\$n" '')
        }
    }
}

Register-Collector -Name 'drivers' -File 'drivers.tsv' -Header "name`tdisplay_name`tstart_mode`ttype`tpath" -Script {
    foreach ($d in @(Get-CimInstance Win32_SystemDriver)) {
        Join-Row $d.Name $d.DisplayName $d.StartMode $d.ServiceType $d.PathName
    }
}

Register-Collector -Name 'network' -File 'network.tsv' -Header "kind`tname`tdetail" -Script {
    # 只记配置。连没连上、此刻的地址、系统代理开没开，都在变，不记。
    foreach ($a in @(Get-NetAdapter -IncludeHidden:$false -ErrorAction SilentlyContinue)) {
        Join-Row 'adapter' $a.Name $a.InterfaceDescription
    }
    $cu = [Microsoft.Win32.Registry]::CurrentUser
    $inet = 'Software\Microsoft\Windows\CurrentVersion\Internet Settings'
    foreach ($v in 'ProxyServer', 'ProxyOverride', 'AutoConfigURL') {
        $val = Get-RegValue $cu $inet $v
        if ($val) { Join-Row 'proxy-setting' $v $val }
    }
    $hosts = Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
    if (Test-Path -LiteralPath $hosts) {
        foreach ($l in [IO.File]::ReadAllLines($hosts)) {
            $t = $l.Trim()
            if ($t -and -not $t.StartsWith('#')) { Join-Row 'hosts' ($t -replace '\s+', ' ') '' }
        }
    }
}

Register-Collector -Name 'fonts' -File 'fonts.tsv' -Header "scope`tfont" -Script {
    $key = 'SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts'
    $roots = [ordered]@{ machine = [Microsoft.Win32.Registry]::LocalMachine; user = [Microsoft.Win32.Registry]::CurrentUser }
    foreach ($scope in $roots.Keys) {
        $k = $roots[$scope].OpenSubKey($key)
        if (-not $k) { continue }
        try { foreach ($n in $k.GetValueNames()) { Join-Row $scope $n } } finally { $k.Dispose() }
    }
}

Register-Collector -Name 'power' -File 'power.txt' -Script {
    # 方案的名字是本地化的，换个语言环境就变样；只记它的编号。
    $out = Invoke-Tool 'powercfg /getactivescheme' 6000
    if ($out -and $out -match '([0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12})') { Join-Row 'active-scheme' $Matches[1].ToLower() }
}
