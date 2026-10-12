# 环境变量和 PATH。名字像密钥的变量，值整个不记。
Register-Collector -Name 'env' -File 'env.tsv' -Header "scope`tname`tvalue" -Script {
    foreach ($scope in 'Machine', 'User') {
        $vars = [Environment]::GetEnvironmentVariables($scope)
        foreach ($k in $vars.Keys) {
            if ($k -ieq 'Path') { continue }
            $v = if (Test-SecretName $k) { '<hidden>' } else { $vars[$k] }
            Join-Row $scope.ToLower() $k $v
        }
    }
}

Register-Collector -Name 'path' -File 'path.tsv' -Sort $false -Header "scope`torder`tentry`texists" -Script {
    # PATH 的先后顺序有意义，所以不排序。
    foreach ($scope in 'Machine', 'User') {
        $raw = [Environment]::GetEnvironmentVariable('Path', $scope)
        if (-not $raw) { continue }
        $i = 0
        foreach ($e in ($raw -split ';')) {
            if ([string]::IsNullOrWhiteSpace($e)) { continue }
            $i++
            $expanded = [Environment]::ExpandEnvironmentVariables($e)
            $exists = if (Test-Path -LiteralPath $expanded) { 'yes' } else { 'missing' }
            Join-Row $scope.ToLower() ('{0:D3}' -f $i) $e $exists
        }
    }
}
