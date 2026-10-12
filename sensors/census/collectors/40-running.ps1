# 注册在系统里的东西：服务、计划任务、开机时启动的程序、在听的端口。
# 只记配置，不记此刻的运行状态；状态一直在变，写进快照会淹没真正的变化。

Register-Collector -Name 'services' -File 'services.tsv' -Header "name`tdisplay_name`tstart_mode`taccount`tpath" -Script {
    foreach ($s in @(Get-CimInstance Win32_Service)) {
        Join-Row $s.Name $s.DisplayName $s.StartMode $s.StartName $s.PathName
    }
}

Register-Collector -Name 'tasks' -File 'tasks.tsv' -Header "task`tenabled`tauthor`taction" -Script {
    foreach ($t in @(Get-ScheduledTask -ErrorAction SilentlyContinue)) {
        $enabled = if ($t.State -eq 'Disabled') { 'no' } else { 'yes' }
        $actions = @(foreach ($a in @($t.Actions)) {
                $p = $a.PSObject.Properties
                if ($p['Execute']) { ("$($a.Execute) $($a.Arguments)").Trim() }
                elseif ($p['ClassId']) { "com:$($a.ClassId)" }
            }) -join ' | '
        Join-Row "$($t.TaskPath)$($t.TaskName)" $enabled $t.Author $actions
    }
}

Register-Collector -Name 'startup' -File 'startup.tsv' -Header "name`twhere`tuser`tcommand" -Script {
    foreach ($s in @(Get-CimInstance Win32_StartupCommand)) {
        Join-Row $s.Name $s.Location $s.User $s.Command
    }
}

Register-Collector -Name 'ports' -File 'ports.tsv' -Header "port`taddress`tprocess" -Script {
    $names = @{}
    foreach ($p in @(Get-Process)) { $names[[int]$p.Id] = $p.ProcessName }
    foreach ($c in @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue)) {
        $pidNum = [int]$c.OwningProcess
        $name = if ($names.ContainsKey($pidNum)) { $names[$pidNum] } else { '?' }
        # 端口补成五位，文本排序才等于数字排序
        Join-Row ('{0:D5}' -f [int]$c.LocalPort) $c.LocalAddress $name
    }
}
