# 配置文件本身。分三种对待：
#   copy   抄一份到 config/ 下，逐行遮掉密钥
#   exists 里面多半有凭据，只记它在不在，不读
# 抄下来的副本是给"细看"那一步读的。

$script:ConfigFiles = @(
    @{ Name = 'pwsh-profile';            Mode = 'copy';   Path = { Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'PowerShell\Microsoft.PowerShell_profile.ps1' } }
    @{ Name = 'pwsh-profile-allhosts';   Mode = 'copy';   Path = { Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'PowerShell\profile.ps1' } }
    @{ Name = 'winps-profile';           Mode = 'copy';   Path = { Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'WindowsPowerShell\Microsoft.PowerShell_profile.ps1' } }
    @{ Name = 'winps-profile-allhosts';  Mode = 'copy';   Path = { Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'WindowsPowerShell\profile.ps1' } }
    @{ Name = 'gitconfig';               Mode = 'copy';   Path = { Join-Path $env:USERPROFILE '.gitconfig' } }
    @{ Name = 'bashrc';                  Mode = 'copy';   Path = { Join-Path $env:USERPROFILE '.bashrc' } }
    @{ Name = 'bash-profile';            Mode = 'copy';   Path = { Join-Path $env:USERPROFILE '.bash_profile' } }
    @{ Name = 'wslconfig';               Mode = 'copy';   Path = { Join-Path $env:USERPROFILE '.wslconfig' } }
    @{ Name = 'windows-terminal';        Mode = 'copy';   Path = { Join-Path $env:LOCALAPPDATA 'Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json' } }
    @{ Name = 'vscode-settings';         Mode = 'copy';   Path = { Join-Path $env:APPDATA 'Code\User\settings.json' } }
    @{ Name = 'cursor-settings';         Mode = 'copy';   Path = { Join-Path $env:APPDATA 'Cursor\User\settings.json' } }
    @{ Name = 'cargo-config';            Mode = 'copy';   Path = { Join-Path $env:USERPROFILE '.cargo\config.toml' } }
    @{ Name = 'pip-ini';                 Mode = 'copy';   Path = { Join-Path $env:APPDATA 'pip\pip.ini' } }
    @{ Name = 'condarc';                 Mode = 'copy';   Path = { Join-Path $env:USERPROFILE '.condarc' } }
    @{ Name = 'claude-global-md';        Mode = 'copy';   Path = { Join-Path $env:USERPROFILE '.claude\CLAUDE.md' } }
    @{ Name = 'npmrc';                   Mode = 'exists'; Path = { Join-Path $env:USERPROFILE '.npmrc' } }
    @{ Name = 'git-credentials';         Mode = 'exists'; Path = { Join-Path $env:USERPROFILE '.git-credentials' } }
    @{ Name = 'ssh-dir';                 Mode = 'exists'; Path = { Join-Path $env:USERPROFILE '.ssh' } }
    @{ Name = 'aws-dir';                 Mode = 'exists'; Path = { Join-Path $env:USERPROFILE '.aws' } }
    @{ Name = 'docker-config';           Mode = 'exists'; Path = { Join-Path $env:USERPROFILE '.docker\config.json' } }
)

Register-Collector -Name 'configs' -File 'configs.tsv' -Header "name`tmode`tstate`tlines`tpath" -Script {
    $dir = Join-Path $script:OutDir 'config'
    if (Test-Path -LiteralPath $dir) { Remove-Item -LiteralPath $dir -Recurse -Force }
    foreach ($c in $script:ConfigFiles) {
        $path = & $c.Path
        if (-not (Test-Path -LiteralPath $path)) { Join-Row $c.Name $c.Mode 'absent' '' $path; continue }
        if ($c.Mode -ne 'copy') { Join-Row $c.Name $c.Mode 'present' '' $path; continue }
        $src = @([IO.File]::ReadAllLines($path))
        $safe = @($src | ForEach-Object { Protect-ConfigLine $_ })
        # 空行在配置文件里有意义，所以不走 Write-Snapshot 的去空行
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        [IO.File]::WriteAllText((Join-Path $dir "$($c.Name).txt"), (($safe -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))
        Join-Row $c.Name $c.Mode 'present' $src.Count $path
    }
}
