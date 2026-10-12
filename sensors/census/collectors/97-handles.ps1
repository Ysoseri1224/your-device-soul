# 把手：这台机器上的程序各自留了哪些"能从外面叫它做事"的口子。
# 第一遍普查记了"装了什么"；这一份记的是"怎么叫它"。三种口子：
#   scheme  程序向系统登记的网址前缀（vscode://、ms-settings: 这类），打开这种网址就是叫那个程序做一件事
#   com     程序向系统登记的自动化接口（Word.Application 这类），脚本里 New-Object -ComObject 就能指挥它
#   cli     能在命令行里直接叫的程序（PATH 上的），连同它自报的版本
#   agent   能不开界面、直接交给它一件活的 AI 工具
# 只读注册表和 PATH，不运行任何东西（agent 和 cli 只问一句版本号）。

Register-Collector -Name 'handles' -File 'handles.tsv' -Header "kind`tname`thow`towner" -Script {
    # —— 网址前缀 ——
    $skipSchemes = 'http', 'https', 'ftp', 'file', 'mailto', 'tel', 'sms', 'res', 'about', 'javascript', 'vbscript', 'data', 'blob', 'local', 'shell', 'search', 'search-ms', 'read', 'ldap', 'news', 'nntp', 'telnet', 'tn3270', 'rlogin', 'wais', 'gopher'
    foreach ($hive in 'Registry::HKEY_CURRENT_USER\Software\Classes', 'Registry::HKEY_LOCAL_MACHINE\Software\Classes') {
        Get-ChildItem -LiteralPath $hive -ErrorAction SilentlyContinue | ForEach-Object {
            $name = $_.PSChildName
            if ($name -notmatch '^[a-z][a-z0-9+.-]{1,40}$' -or $skipSchemes -contains $name) { return }
            if ($null -eq $_.GetValue('URL Protocol')) { return }
            $k = $_.OpenSubKey('shell\open\command'); $cmd = if ($k) { "$($k.GetValue(''))" } else { '' }
            $exe = if ($cmd -match '^"([^"]+)"' -or $cmd -match '^(\S+)') { [IO.Path]::GetFileName($Matches[1]) } else { '' }
            if ($exe) { Join-Row 'scheme' "${name}:" "Start-Process '${name}:…'" $exe }
        }
    }
    # —— 自动化接口：只列常见的、确实用来指挥程序的那一类（名字以 .Application 结尾） ——
    Get-ChildItem -LiteralPath 'Registry::HKEY_LOCAL_MACHINE\Software\Classes' -ErrorAction SilentlyContinue | Where-Object { $_.PSChildName -match '^[A-Za-z][A-Za-z0-9]*\.Application$' } | ForEach-Object {
        $k = $_.OpenSubKey('CLSID'); $clsid = if ($k) { "$($k.GetValue(''))" } else { '' }
        $s = if ($clsid) { [Microsoft.Win32.Registry]::ClassesRoot.OpenSubKey("CLSID\$clsid\LocalServer32") }
        $server = if ($s) { "$($s.GetValue(''))" } else { '' }
        $exe = if ($server -match '^"?([^"]+?\.exe)') { [IO.Path]::GetFileName($Matches[1]) } else { '' }
        if ($exe) { Join-Row 'com' $_.PSChildName "New-Object -ComObject $($_.PSChildName)" $exe }
    }
    # —— 命令行里叫得动的：只列一批常用来"让程序做事"的，在 PATH 上找得到才记 ——
    $clis = 'git', 'gh', 'code', 'cursor', 'docker', 'wsl', 'winget', 'scoop', 'choco', 'npm', 'pnpm', 'node', 'python', 'pip', 'uv', 'cargo', 'go', 'java', 'ffmpeg', 'magick', 'pandoc', '7z', 'curl', 'ssh', 'scp', 'rclone', 'yt-dlp', 'adb', 'kubectl', 'terraform', 'az', 'aws', 'gcloud', 'typora', 'obsidian', 'everything', 'es'
    foreach ($c in $clis) {
        $found = Get-Command $c -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) { Join-Row 'cli' $c "$c …" ([IO.Path]::GetFileName($found.Source)) }
    }
    # —— 能直接交活的 AI 工具 ——
    $agents = [ordered]@{ claude = 'claude -p "<要它做的事>"'; codex = 'codex exec "<要它做的事>"'; gemini = 'gemini -p "<要它做的事>"'; 'cursor-agent' = 'cursor-agent -p "<要它做的事>"'; aider = 'aider --message "<要它做的事>"' }
    foreach ($a in $agents.Keys) {
        $found = Get-Command $a -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) { Join-Row 'agent' $a $agents[$a] ([IO.Path]::GetFileName($found.Source)) }
    }
}
