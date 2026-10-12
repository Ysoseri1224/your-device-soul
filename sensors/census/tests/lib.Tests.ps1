# 本机自带的是 Pester 3.4，所以用旧写法（Should Be）。
. (Join-Path $PSScriptRoot '..\lib.ps1')

Describe 'Protect-Secret' {
    It '遮掉 sk- 开头的密钥' {
        Protect-Secret 'key is sk-abcdefghijklmnopqrstuv end' | Should Be 'key is <hidden> end'
    }
    It '遮掉 GitHub 令牌' {
        Protect-Secret 'ghp_abcdefghijklmnopqrstuvwxyz0123' | Should Be '<hidden>'
    }
    It '遮掉网址里的用户名和密码，保留其余部分' {
        Protect-Secret 'https://bob:hunter2@example.com/x.git' | Should Be 'https://<hidden>@example.com/x.git'
    }
    It '遮掉 token= 后面的值，保留前面的名字' {
        Protect-Secret 'run --token=abcdef123456 --fast' | Should Be 'run --token=<hidden> --fast'
    }
    It '只是出现了 Password 这个词、后面没有等号或冒号的，不遮' {
        Protect-Secret 'Advanced Archive Password Recovery.lnk' | Should Be 'Advanced Archive Password Recovery.lnk'
    }
    It 'JSON 里 "token": "值" 的写法照样遮' {
        Protect-Secret '"token": "abcdef123456"' | Should Be '"token": "<hidden>"'
    }
    It '普通文字原样返回' {
        Protect-Secret 'C:\Program Files\Git\cmd\git.exe' | Should Be 'C:\Program Files\Git\cmd\git.exe'
    }
    It '空值返回空串' {
        Protect-Secret $null | Should Be ''
    }
}

Describe 'Test-SecretName' {
    It '名字里带 TOKEN 的算' { Test-SecretName 'OPENAI_API_TOKEN' | Should Be $true }
    It '名字里带 key 的算' { Test-SecretName 'ANTHROPIC_API_KEY' | Should Be $true }
    It 'PATH 不算' { Test-SecretName 'PATH' | Should Be $false }
}

Describe 'Join-Row' {
    It '用制表符拼接，字段里的制表符和换行换成空格' {
        Join-Row 'a' "b`tc" "d`r`ne" | Should Be "a`tb c`td e"
    }
    It '空值变成空字段' {
        Join-Row 'a' $null 'c' | Should Be "a`t`tc"
    }
    It '拼接后同样会遮掉密钥' {
        Join-Row 'svc' 'app.exe --password=supersecret1' | Should Be "svc`tapp.exe --password=<hidden>"
    }
}

Describe 'Write-Snapshot' {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("ysoul-census-test-" + [guid]::NewGuid().ToString('N'))
    $file = Join-Path $tmp 'sub\x.tsv'

    It '排序、去重、去掉空行，返回行数' {
        $n = Write-Snapshot -Path $file -Lines @('b', 'a', '', 'b', '  ')
        $n | Should Be 2
        [IO.File]::ReadAllText($file) | Should Be "a`nb`n"
    }
    It '表头放在最前，不参与排序' {
        Write-Snapshot -Path $file -Lines @('z', 'a') -Header 'name' | Out-Null
        [IO.File]::ReadAllText($file) | Should Be "name`na`nz`n"
    }
    It 'Sort 为假时保持原顺序' {
        Write-Snapshot -Path $file -Lines @('z', 'a') -Sort $false | Out-Null
        [IO.File]::ReadAllText($file) | Should Be "z`na`n"
    }
    It '写出的文件没有 BOM，也没有回车' {
        Write-Snapshot -Path $file -Lines @('a') | Out-Null
        $bytes = [IO.File]::ReadAllBytes($file)
        $bytes[0] | Should Be 97
        ($bytes -contains 13) | Should Be $false
    }
    It '同样的输入写两次，文件完全一样' {
        Write-Snapshot -Path $file -Lines @('b', 'a', 'c') | Out-Null
        $first = [IO.File]::ReadAllText($file)
        Write-Snapshot -Path $file -Lines @('c', 'b', 'a') | Out-Null
        [IO.File]::ReadAllText($file) | Should Be $first
    }

    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

Describe 'Register-Collector' {
    It '登记之后能在表里找到' {
        Register-Collector -Name 'demo' -File 'demo.txt' -Script { 'x' }
        $script:Collectors.Contains('demo') | Should Be $true
        $script:Collectors['demo'].File | Should Be 'demo.txt'
    }
}

Describe 'Protect-ConfigLine' {
    It 'JSON 里名字像密钥的一项，值整个遮掉，保留结尾的逗号' {
        Protect-ConfigLine '  "apiKey": "abc123",' | Should Be '  "apiKey": <hidden>,'
    }
    It '等号写法同样处理' {
        Protect-ConfigLine 'auth_token = xyz' | Should Be 'auth_token = <hidden>'
    }
    It '普通设置原样保留' {
        Protect-ConfigLine '  "editor.fontSize": 14,' | Should Be '  "editor.fontSize": 14,'
    }
    It '不是"名字: 值"的行，按普通文字遮' {
        Protect-ConfigLine 'curl https://bob:pw123456@host/x' | Should Be 'curl https://<hidden>@host/x'
    }
    It '空行返回空串' {
        Protect-ConfigLine '' | Should Be ''
    }
}

Describe 'Get-RegValue 与 Get-RegSubKeyNames' {
    $cu = [Microsoft.Win32.Registry]::CurrentUser
    It '读得到一个一定存在的位置' {
        @(Get-RegSubKeyNames $cu 'Software').Count -gt 0 | Should Be $true
    }
    It '位置不存在时返回空，不报错' {
        @(Get-RegSubKeyNames $cu 'Software\ysoul-no-such-key-xyz').Count | Should Be 0
        Get-RegValue $cu 'Software\ysoul-no-such-key-xyz' 'x' | Should Be $null
    }
}
