# 设置一层：每样东西被调成了什么样。
# 资源管理器的选项、外观、桌面的布置、各程序留下配置的地方、编辑器插件、AI 工具的配置。

Register-Collector -Name 'explorer-settings' -File 'explorer-settings.tsv' -Header "name`tvalue" -Script {
    $cu = [Microsoft.Win32.Registry]::CurrentUser
    $key = 'Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced'
    $k = $cu.OpenSubKey($key)
    if ($k) {
        try {
            foreach ($n in $k.GetValueNames()) {
                $v = $k.GetValue($n)
                if ($v -is [byte[]]) { continue }
                Join-Row $n $v
            }
        } finally { $k.Dispose() }
    }
}

Register-Collector -Name 'personalization' -File 'personalization.tsv' -Header "name`tvalue" -Script {
    $cu = [Microsoft.Win32.Registry]::CurrentUser
    $theme = 'Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'
    foreach ($n in 'AppsUseLightTheme', 'SystemUsesLightTheme', 'EnableTransparency') {
        $v = Get-RegValue $cu $theme $n
        if ($null -ne $v) { Join-Row $n $v }
    }
    $accent = Get-RegValue $cu 'Software\Microsoft\Windows\DWM' 'AccentColor'
    if ($null -ne $accent) {
        # 系统存的是 ABGR，转成常见的 #RRGGBB
        $n = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]$accent), 0)
        Join-Row 'AccentColor' ('#{0:X2}{1:X2}{2:X2}' -f ($n -band 0xFF), (($n -shr 8) -band 0xFF), (($n -shr 16) -band 0xFF))
    }
    $wall = Get-RegValue $cu 'Control Panel\Desktop' 'WallPaper'
    if ($wall) { Join-Row 'Wallpaper' $wall }
    $k = $cu.OpenSubKey('Keyboard Layout\Preload')
    if ($k) {
        try { foreach ($n in ($k.GetValueNames() | Sort-Object)) { Join-Row "KeyboardLayout$n" $k.GetValue($n) } } finally { $k.Dispose() }
    }
}

Register-Collector -Name 'desktop' -File 'desktop.tsv' -Header "where`tname" -Script {
    $desk = [Environment]::GetFolderPath('Desktop')
    Join-Row 'desktop-folder' $desk
    if (Test-Path -LiteralPath $desk) {
        foreach ($i in @(Get-ChildItem -LiteralPath $desk -Force -ErrorAction SilentlyContinue)) {
            $kind = if ($i.PSIsContainer) { 'desktop-dir' } else { 'desktop-file' }
            # 快捷方式和程序的名字说明机器上有什么，照记。文档的名字常带人名和私事，只记类型。
            $name = if ($i.PSIsContainer -or $i.Extension -in '.lnk', '.url', '.exe', '.ahk', '.bat', '.cmd', '.ps1') { $i.Name } else { '<文档>' + $i.Extension.ToLowerInvariant() }
            Join-Row $kind $name
        }
    }
    $pins = Join-Path $env:APPDATA 'Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar'
    if (Test-Path -LiteralPath $pins) {
        foreach ($i in @(Get-ChildItem -LiteralPath $pins -Filter *.lnk -ErrorAction SilentlyContinue)) { Join-Row 'taskbar-pin' $i.BaseName }
    }
    foreach ($menu in [Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('CommonPrograms')) {
        if (-not (Test-Path -LiteralPath $menu)) { continue }
        foreach ($i in @(Get-ChildItem -LiteralPath $menu -Filter *.lnk -Recurse -ErrorAction SilentlyContinue)) {
            Join-Row 'start-menu' ($i.FullName.Substring($menu.Length).TrimStart('\') -replace '\.lnk$', '')
        }
    }
}

Register-Collector -Name 'footprint' -File 'footprint.tsv' -Header "where`tname" -Script {
    # 哪些程序在这几个地方留下了自己的目录或文件。只记名字，不进去看。
    $places = [ordered]@{
        'home'    = $env:USERPROFILE
        'roaming' = $env:APPDATA
        'local'   = $env:LOCALAPPDATA
        'config'  = (Join-Path $env:USERPROFILE '.config')
    }
    foreach ($w in $places.Keys) {
        $p = $places[$w]
        if (-not (Test-Path -LiteralPath $p)) { continue }
        foreach ($i in @(Get-ChildItem -LiteralPath $p -Force -ErrorAction SilentlyContinue)) {
            if ($w -eq 'home' -and -not $i.PSIsContainer -and -not $i.Name.StartsWith('.')) { continue }
            if ($i.Name -match '^(NTUSER|ntuser)\.') { continue }
            Join-Row $w $(if ($i.PSIsContainer) { "$($i.Name)\" } else { $i.Name })
        }
    }
}

Register-Collector -Name 'editor-extensions' -File 'editor-extensions.tsv' -Header "editor`textension" -Script {
    foreach ($ed in 'code', 'cursor') {
        if (-not (Get-Command $ed -CommandType Application -ErrorAction SilentlyContinue)) { continue }
        $out = Invoke-Tool "$ed --list-extensions --show-versions" 30000
        if ($out) { foreach ($l in ($out -split "`r?`n")) { if ($l.Trim() -match '^[\w.\-]+@[\w.\-]+$') { Join-Row $ed $l.Trim() } } }
    }
}

Register-Collector -Name 'ai-tools' -File 'ai-tools.tsv' -Header "tool`tkind`tname" -Script {
    # 各 AI 工具配了什么。只记名字和设置项的名字，不记值。
    $claude = Join-Path $env:USERPROFILE '.claude'
    foreach ($kind in 'rules', 'skills', 'agents', 'commands', 'hooks', 'plugins') {
        $dir = Join-Path $claude $kind
        if (-not (Test-Path -LiteralPath $dir)) { continue }
        foreach ($i in @(Get-ChildItem -LiteralPath $dir -ErrorAction SilentlyContinue)) { Join-Row 'claude' $kind $i.Name }
    }
    $jsons = [ordered]@{
        'claude'    = (Join-Path $claude 'settings.json')
        'claude-app' = (Join-Path $env:USERPROFILE '.claude.json')
        'cc-switch' = (Join-Path $env:USERPROFILE '.cc-switch\settings.json')
    }
    foreach ($tool in $jsons.Keys) {
        $f = $jsons[$tool]
        if (-not (Test-Path -LiteralPath $f)) { continue }
        try {
            $j = Get-Content -LiteralPath $f -Raw | ConvertFrom-Json -AsHashtable
            foreach ($k in $j.Keys) { Join-Row $tool 'setting' $k }
            if ($j.ContainsKey('mcpServers') -and $j['mcpServers'] -is [System.Collections.IDictionary]) {
                foreach ($m in $j['mcpServers'].Keys) { Join-Row $tool 'mcp-server' $m }
            }
        } catch { Join-Row $tool 'setting' '(读不出来)' }
    }
    foreach ($tool in '.codex', '.gemini', '.cursor', '.clawd') {
        $dir = Join-Path $env:USERPROFILE $tool
        if (Test-Path -LiteralPath $dir) {
            foreach ($i in @(Get-ChildItem -LiteralPath $dir -ErrorAction SilentlyContinue)) {
                Join-Row $tool.TrimStart('.') 'entry' $(if ($i.PSIsContainer) { "$($i.Name)\" } else { $i.Name })
            }
        }
    }
}
