# 看一条命令：它是不是简单到程序能完全看懂。
# 用 PowerShell 自己的语法分析器把命令拆开，不执行它。从标准输入读命令，往标准输出写一行 JSON。
# "简单"的意思：只有一条管道；管道第一段是一条命令，参数全是写死的值；后面几段只做筛选和排版；
# 哪里都没有变量、方法调用、类型转换、子表达式、重定向、套着的命令。
# 不满足的不是"坏命令"，只是程序看不懂，要交给机主过目。
$ErrorActionPreference = 'Stop'
# 输入输出直接按字节读写、按 UTF-8 算：桌面上那只没有控制台，去设控制台的编码会出错。
function ReadAll { $r = New-Object IO.StreamReader ([Console]::OpenStandardInput()), ([Text.Encoding]::UTF8); $r.ReadToEnd() }
function Done($o) { $b = [Text.Encoding]::UTF8.GetBytes(($o | ConvertTo-Json -Depth 6 -Compress) + "`n"); $s = [Console]::OpenStandardOutput(); $s.Write($b, 0, $b.Length); $s.Flush(); exit 0 }
$cmd = ReadAll
function No($why) { Done @{ simple = $false; why = $why } }

$L = 'System.Management.Automation.Language'
$tokens = $null; $errs = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput($cmd, [ref]$tokens, [ref]$errs)
if ($errs.Count) { No "语法有错：$($errs[0].Message)" }
if ($ast.ParamBlock -or $ast.BeginBlock -or $ast.ProcessBlock -or $ast.DynamicParamBlock) { No '不是一条简单的命令' }
$st = @($ast.EndBlock.Statements)
if ($st.Count -ne 1) { No '不止一条语句' }
$pipe = $st[0]
if ($pipe.GetType().FullName -ne "$L.PipelineAst") { No '有赋值、判断、循环或函数定义' }
if ($pipe.Background) { No '要在后台运行' }

$alias = @{ gci = 'get-childitem'; ls = 'get-childitem'; dir = 'get-childitem'; gc = 'get-content'; cat = 'get-content'; type = 'get-content'
  gi = 'get-item'; gp = 'get-itemproperty'; sls = 'select-string'; ps = 'get-process'; gps = 'get-process'; gsv = 'get-service'; gcm = 'get-command'
  rvpa = 'resolve-path'; gcim = 'get-ciminstance'; select = 'select-object'; where = 'where-object'; '?' = 'where-object'; sort = 'sort-object'
  measure = 'measure-object'; group = 'group-object'; ft = 'format-table'; fl = 'format-list'; 'git.exe' = 'git' }

$names = @()
foreach ($el in $pipe.PipelineElements) {
  if ($el.GetType().FullName -ne "$L.CommandAst") { No '管道里有不是命令的东西' }
  if ($el.Redirections.Count) { No '有重定向' }
  if ("$($el.InvocationOperator)" -ne 'Unknown') { No '用了调用运算符' }
  $n = $el.GetCommandName(); if (-not $n) { No '命令名不是写死的' }
  $n = $n.ToLower(); if ($alias.ContainsKey($n)) { $n = $alias[$n] }
  $names += $n
}

$all = @($ast.FindAll({ param($a) $a.GetType().FullName -eq "$L.CommandAst" }, $true))
if ($all.Count -ne $pipe.PipelineElements.Count) { No '命令里还套着命令' }

$forbidden = 'InvokeMemberExpressionAst', 'TypeExpressionAst', 'ConvertExpressionAst', 'SubExpressionAst', 'UsingExpressionAst', 'AssignmentStatementAst', 'FileRedirectionAst', 'MergingRedirectionAst', 'ExpandableStringExpressionAst', 'ArrayExpressionAst', 'AttributedExpressionAst', 'FunctionDefinitionAst', 'TrapStatementAst'
$bad = @($ast.FindAll({ param($a) $forbidden -contains $a.GetType().Name }, $true))
if ($bad.Count) { No "用了程序看不懂的写法（$($bad[0].GetType().Name -replace 'Ast$', '')）" }

$vars = @($ast.FindAll({ param($a) $a.GetType().Name -eq 'VariableExpressionAst' }, $true) | Where-Object { @('_', 'psitem', 'true', 'false', 'null') -notcontains $_.VariablePath.UserPath.ToLower() })
if ($vars.Count) { No "用了变量 `$$($vars[0].VariablePath.UserPath)" }

$ops = @($ast.FindAll({ param($a) $a.GetType().Name -eq 'BinaryExpressionAst' }, $true) | Where-Object { "$($_.Operator)" -notmatch '^[IC]?(eq|ne|gt|lt|ge|le|like|notlike|match|notmatch|contains|notcontains|in|notin)$|^(And|Or|Xor)$' })
if ($ops.Count) { No "用了拼接或运算（$($ops[0].Operator)）" }

$src = $pipe.PipelineElements[0]
$inSrc = @($src.FindAll({ param($a) @('ScriptBlockExpressionAst', 'HashtableAst', 'ParenExpressionAst', 'VariableExpressionAst', 'MemberExpressionAst') -contains $a.GetType().Name }, $true))
if ($inSrc.Count) { No '第一条命令的参数不是写死的值' }

function Const($e) {
  $t = $e.GetType().Name
  if ($t -eq 'StringConstantExpressionAst') { return , @("$($e.Value)") }
  if ($t -eq 'ConstantExpressionAst') { return , @("$($e.Value)") }
  if ($t -eq 'ArrayLiteralAst') { $o = @(); foreach ($x in $e.Elements) { $v = Const $x; if ($null -eq $v) { return $null }; $o += $v }; return , $o }
  return $null
}

$args = @(); $words = @(); $pending = $null
$els = @($src.CommandElements)
for ($i = 1; $i -lt $els.Count; $i++) {
  $e = $els[$i]
  if ($e.GetType().Name -eq 'CommandParameterAst') {
    $words += $e.Extent.Text
    $p = $e.ParameterName.ToLower()
    if ($e.Argument) {
      $v = Const $e.Argument; if ($null -eq $v) { No '参数不是写死的值' }
      foreach ($x in $v) { $args += @{ param = $p; value = $x } }
      $pending = $null
    } else { $args += @{ param = $p; value = $null }; $pending = $p }
    continue
  }
  $v = Const $e; if ($null -eq $v) { No '参数不是写死的值' }
  foreach ($x in $v) { $args += @{ param = $pending; value = $x }; $words += $x }
  $pending = $null
}

$filters = @()
for ($k = 1; $k -lt $pipe.PipelineElements.Count; $k++) {
  $f = $pipe.PipelineElements[$k]; $ps = @(); $pos = 0; $fe = @($f.CommandElements)
  for ($i = 1; $i -lt $fe.Count; $i++) {
    if ($fe[$i].GetType().Name -eq 'CommandParameterAst') { $ps += $fe[$i].ParameterName.ToLower() }
    elseif ($i -eq 1 -or $fe[$i - 1].GetType().Name -ne 'CommandParameterAst' -or $fe[$i - 1].Argument) { $pos++ }
  }
  $filters += @{ name = $names[$k]; params = $ps; positionals = $pos }
}

Done @{ simple = $true; why = ''; source = @{ name = $names[0]; args = $args; words = $words }; filters = $filters }
