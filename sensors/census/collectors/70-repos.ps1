# 盘上的每一个仓库。位置和远端是稳定的，记在 repos.tsv；
# 当前分支、最后一次提交、有多少没提交的改动天天变，单独记在 repos-state.tsv。
$script:RepoSkip = @(
    'Windows', 'Program Files', 'Program Files (x86)', 'ProgramData', '$Recycle.Bin', 'System Volume Information',
    'node_modules', 'AppData', 'target', '.venv', 'venv', '__pycache__', '.cargo', '.rustup', '.npm', '.cache', 'Temp', 'tmp'
)

function Find-Repos {
    param([string]$Root, [int]$MaxDepth)
    $opt = [IO.EnumerationOptions]::new()
    $opt.IgnoreInaccessible = $true
    $opt.AttributesToSkip = [IO.FileAttributes]::ReparsePoint
    $queue = [Collections.Generic.Queue[object]]::new()
    $queue.Enqueue([pscustomobject]@{ Dir = $Root; Depth = 0 })
    while ($queue.Count) {
        $item = $queue.Dequeue()
        $dir = [string]$item.Dir
        $depth = [int]$item.Depth
        if (Test-Path -LiteralPath (Join-Path $dir '.git')) { $dir; continue }
        if ($depth -ge $MaxDepth) { continue }
        try {
            foreach ($sub in [IO.Directory]::EnumerateDirectories($dir, '*', $opt)) {
                if ($script:RepoSkip -contains [IO.Path]::GetFileName($sub)) { continue }
                $queue.Enqueue([pscustomobject]@{ Dir = $sub; Depth = $depth + 1 })
            }
        } catch { }
    }
}

$script:RepoCache = $null
function Get-RepoList {
    if ($null -eq $script:RepoCache) {
        $script:RepoCache = @(foreach ($d in [IO.DriveInfo]::GetDrives()) {
                if ($d.DriveType -eq 'Fixed' -and $d.IsReady) { Find-Repos $d.Name $script:RepoDepth }
            })
    }
    $script:RepoCache
}

function Invoke-Git {
    param([string]$Repo, [string]$GitArgs)
    $out = Invoke-Tool "git -c safe.directory=* -C `"$Repo`" $GitArgs" 8000
    if ($null -eq $out) { return '' }
    return $out.Trim()
}

Register-Collector -Name 'repos' -File 'repos.tsv' -Header "path`tremote" -Script {
    foreach ($r in (Get-RepoList)) { Join-Row $r (Invoke-Git $r 'remote get-url origin 2>nul') }
}

Register-Collector -Name 'repos-state' -File 'repos-state.tsv' -Header "path`tbranch`tlast_commit`tuncommitted" -Script {
    foreach ($r in (Get-RepoList)) {
        $branch = Invoke-Git $r 'rev-parse --abbrev-ref HEAD 2>nul'
        $last = Invoke-Git $r 'log -1 --format=%cs 2>nul'
        $dirty = @((Invoke-Git $r 'status --porcelain 2>nul') -split "`r?`n" | Where-Object { $_ }).Count
        Join-Row $r $branch $last $dirty
    }
}
