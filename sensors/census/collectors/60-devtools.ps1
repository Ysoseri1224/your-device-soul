# 开发环境：有哪些工具、各是什么版本、装在哪。
$script:DevTools = [ordered]@{
    git = '--version'; gh = '--version'; node = '--version'; npm = '--version'; pnpm = '--version'; yarn = '--version'
    bun = '--version'; deno = '--version'; python = '--version'; py = '--version'; pip = '--version'; uv = '--version'
    rustc = '--version'; cargo = '--version'; rustup = '--version'; go = 'version'; java = '-version'; dotnet = '--version'
    docker = '--version'; pwsh = '--version'; code = '--version'; cursor = '--version'; claude = '--version'; codex = '--version'
    cmake = '--version'; gcc = '--version'; clang = '--version'; ffmpeg = '-version'; tsc = '--version'; scoop = '--version'
    winget = '--version'; choco = '--version'
}

Register-Collector -Name 'devtools' -File 'devtools.tsv' -Header "tool`tversion`tpath" -Script {
    foreach ($tool in $script:DevTools.Keys) {
        $cmd = Get-Command $tool -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $cmd) { continue }
        $out = Invoke-Tool "$tool $($script:DevTools[$tool])"
        $first = if ($out) { ($out -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -First 1) } else { '(no answer)' }
        Join-Row $tool $first $cmd.Source
    }
}

Register-Collector -Name 'global-packages' -File 'global-packages.tsv' -Header "manager`tpackage" -Script {
    if (Get-Command npm -ErrorAction SilentlyContinue) {
        $out = Invoke-Tool 'npm ls -g --depth=0 --parseable --long' 20000
        if ($out) {
            foreach ($l in ($out -split "`r?`n")) {
                # 每行形如 路径:包名@版本
                if ($l -match ':([^:\\/]+@[^:]+)$') { Join-Row 'npm' $Matches[1] }
            }
        }
    }
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        $out = Invoke-Tool 'cargo install --list' 15000
        if ($out) {
            foreach ($l in ($out -split "`r?`n")) { if ($l -match '^(\S+ v[^\s:]+)') { Join-Row 'cargo' $Matches[1] } }
        }
    }
    if (Get-Command wsl -ErrorAction SilentlyContinue) {
        $out = Invoke-Tool 'wsl -l -q' 8000 ([Text.Encoding]::Unicode)
        if ($out) {
            foreach ($l in ($out -split "`r?`n")) { $n = $l.Trim([char]0, ' '); if ($n) { Join-Row 'wsl' $n } }
        }
    }
}
