# 界面操作的手：经由 Windows 自带的"界面自动化"（给读屏软件用的那一套）去看、去点别的程序里的东西。
# 从标准输入读一行 JSON（op、hwnd、idx、name、action、text），往标准输出写一行 JSON。
# 只对指定的那一个窗口动手。密码框不读也不写。
$ErrorActionPreference = 'Stop'
# 输入输出直接按字节读写、按 UTF-8 算：桌面上那只没有控制台，去设控制台的编码会出错。
function ReadAll { $r = New-Object IO.StreamReader ([Console]::OpenStandardInput()), ([Text.Encoding]::UTF8); $r.ReadToEnd() }
function Done($o) { $b = [Text.Encoding]::UTF8.GetBytes(($o | ConvertTo-Json -Depth 6 -Compress) + "`n"); $s = [Console]::OpenStandardOutput(); $s.Write($b, 0, $b.Length); $s.Flush(); exit 0 }
function Fail($why) { Done @{ ok = $false; why = $why } }
try {
  $q = (ReadAll) | ConvertFrom-Json
  Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
  Add-Type -Namespace Y -Name M -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern bool GetCursorPos(out long p);
[DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, UIntPtr e);
'@
  [void][Y.M]::SetProcessDPIAware()
  $AE = [System.Windows.Automation.AutomationElement]
  $CT = [System.Windows.Automation.ControlType]
  $win = $AE::FromHandle([IntPtr][long]$q.hwnd)
  if (-not $win) { Fail '找不到那个窗口' }

  function Rect($r) { if ($r.IsEmpty -or [double]::IsInfinity($r.Width)) { return $null }; , @([int]$r.Left, [int]$r.Top, [int]$r.Right, [int]$r.Bottom) }
  function Kind($e) { $e.Current.ControlType.ProgrammaticName -replace '^ControlType\.', '' }

  # 能动手的东西：按钮、输入框、菜单项、页签、链接、勾选框、下拉框、列表项。只要看得见、能用的。
  function Things {
    $types = $CT::Button, $CT::Edit, $CT::Document, $CT::MenuItem, $CT::TabItem, $CT::Hyperlink, $CT::CheckBox, $CT::RadioButton, $CT::ComboBox, $CT::ListItem, $CT::TreeItem, $CT::SplitButton
    $any = New-Object System.Windows.Automation.OrCondition (, [System.Windows.Automation.Condition[]]($types | ForEach-Object { New-Object System.Windows.Automation.PropertyCondition $AE::ControlTypeProperty, $_ }))
    $cond = New-Object System.Windows.Automation.AndCondition $any, (New-Object System.Windows.Automation.PropertyCondition $AE::IsOffscreenProperty, $false), (New-Object System.Windows.Automation.PropertyCondition $AE::IsEnabledProperty, $true)
    $found = @($win.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond))
    # 右键菜单、下拉菜单是另一个顶层窗口：同一个程序弹出来的也算进来，排在后面
    $mine = New-Object System.Windows.Automation.PropertyCondition $AE::ProcessIdProperty, $win.Current.ProcessId
    foreach ($top in @($AE::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children, $mine))) {
      if ($top.Current.ControlType -eq $CT::Menu -or $top.Current.ClassName -eq '#32768') { $found += @($top.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)) }
    }
    $out = New-Object System.Collections.ArrayList
    foreach ($e in $found) {
      $r = Rect $e.Current.BoundingRectangle; if (-not $r) { continue }
      $n = "$($e.Current.Name)".Trim(); $k = Kind $e
      if (-not $n -and @('Edit', 'Document', 'ComboBox') -notcontains $k) { continue }      # 没名字的按钮说不清是什么，不列
      [void]$out.Add($e); if ($out.Count -ge 400) { break }
    }
    , $out
  }
  function Describe($e, $i) {
    $n = "$($e.Current.Name)".Trim(); if ($n.Length -gt 60) { $n = $n.Substring(0, 60) }
    @{ i = $i; kind = (Kind $e); name = $n; rect = (Rect $e.Current.BoundingRectangle); secret = [bool]$e.Current.IsPassword; focus = [bool]$e.Current.HasKeyboardFocus }
  }
  function TextHolders {
    $cond = New-Object System.Windows.Automation.PropertyCondition $AE::IsTextPatternAvailableProperty, $true
    $all = @($win.FindAll([System.Windows.Automation.TreeScope]::Subtree, $cond)) | Where-Object { -not $_.Current.IsPassword -and -not $_.Current.IsOffscreen }
    # 正在输入的那个排最前，其余按大小排
    , @($all | Sort-Object @{ e = { -[int]$_.Current.HasKeyboardFocus } }, @{ e = { -($_.Current.BoundingRectangle.Width * $_.Current.BoundingRectangle.Height) } })
  }

  switch ($q.op) {
    'see' {
      $list = Things; $i = 0
      Done @{ ok = $true; title = "$($win.Current.Name)"; things = @($list | ForEach-Object { Describe $_ $i; $i++ }) }
    }
    'read' {
      foreach ($e in (TextHolders)) {
        $p = $e.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
        $t = $p.DocumentRange.GetText(6000)
        if ($t.Trim()) { Done @{ ok = $true; from = "$($e.Current.Name)"; text = $t } }
      }
      # 有些老式的编辑器不支持按段取字，但会把整篇文字当成那一块的"值"或"名字"交出来
      $kids = @($win.FindAll([System.Windows.Automation.TreeScope]::Subtree, [System.Windows.Automation.Condition]::TrueCondition)) | Where-Object { -not $_.Current.IsPassword -and -not $_.Current.IsOffscreen }
      foreach ($e in ($kids | Sort-Object @{ e = { -[int]$_.Current.HasKeyboardFocus } })) {
        $vp = $null; $t = ''
        if ($e.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$vp)) { $t = "$($vp.Current.Value)" }
        if ($t.Trim().Length -lt 20 -and @('Pane', 'Document', 'Edit') -contains (Kind $e)) { $t = "$($e.Current.Name)" }
        if ($t.Trim().Length -ge 20) { if ($t.Length -gt 6000) { $t = $t.Substring(0, 6000) }; Done @{ ok = $true; from = (Kind $e); text = $t } }
      }
      Fail '这个窗口里没有读得到的文字（有些程序不把文字交出来，那就只能 look）'
    }
    'selection' {
      foreach ($e in (TextHolders)) {
        $p = $e.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
        foreach ($range in @($p.GetSelection())) {
          $t = $range.GetText(2000)
          if ($t.Trim()) { Done @{ ok = $true; text = $t } }
        }
      }
      Done @{ ok = $true; text = '' }
    }
    'find' {
      foreach ($e in (TextHolders)) {
        $p = $e.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
        $range = $p.DocumentRange.FindText([string]$q.text, $false, $true)
        if (-not $range) { continue }
        $rs = @($range.GetBoundingRectangles()); if (-not $rs.Count) { Done @{ ok = $true; rect = $null } }     # 有这段字，但滚出去了
        $l = ($rs | Measure-Object Left -Minimum).Minimum; $t = ($rs | Measure-Object Top -Minimum).Minimum
        $r = ($rs | Measure-Object Right -Maximum).Maximum; $b = ($rs | Measure-Object Bottom -Maximum).Maximum
        Done @{ ok = $true; rect = @([int]$l, [int]$t, [int]$r, [int]$b) }
      }
      Fail '没在这个窗口里找到这段字'
    }
    'scroll' {
      # 滚动：不用先点名哪一项。把焦点放到窗口里最大的那块正文上（没有就放到窗口上），按对应的键
      $where = @{ 'bottom' = '^{END}'; 'top' = '^{HOME}'; 'down' = '{PGDN}'; 'up' = '{PGUP}' }
      $k = "$($q.text)".ToLower().Trim(); if (-not $where.ContainsKey($k)) { Fail "滚到哪只有四种：bottom、top、down、up" }
      $docs = @($win.FindAll([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition $AE::ControlTypeProperty, $CT::Document))) | Where-Object { -not $_.Current.IsOffscreen }
      $t = $docs | Sort-Object { -($_.Current.BoundingRectangle.Width * $_.Current.BoundingRectangle.Height) } | Select-Object -First 1
      if (-not $t) { $t = $win }
      try { $t.SetFocus() } catch { $win.SetFocus() }
      Start-Sleep -Milliseconds 150
      Add-Type -AssemblyName System.Windows.Forms
      [System.Windows.Forms.SendKeys]::SendWait($where[$k]); Start-Sleep -Milliseconds 350
      Done @{ ok = $true; how = "滚到了 $k" }
    }
    'do' {
      $list = Things
      if ($q.idx -lt 0 -or $q.idx -ge $list.Count) { Fail '界面变了，那一项不在了。重新看一遍再动手' }
      $e = $list[[int]$q.idx]; $n = "$($e.Current.Name)".Trim(); if ($n.Length -gt 60) { $n = $n.Substring(0, 60) }
      if ($n -ne "$($q.name)" -or (Kind $e) -ne "$($q.kind)") { Fail '界面变了，那一项现在是别的东西。重新看一遍再动手' }
      if ($e.Current.IsPassword) { Fail '这是密码框，不碰' }
      $pat = $null
      switch ($q.action) {
        'focus' { $e.SetFocus(); Done @{ ok = $true; how = '焦点放过去了' } }
        'set' {
          if (-not $e.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pat)) { Fail '这一项不能直接填字' }
          if ($pat.Current.IsReadOnly) { Fail '这一项是只读的' }
          $pat.SetValue([string]$q.text); Done @{ ok = $true; how = '填进去了' }
        }
        'click' {
          if ($e.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pat)) { $pat.Invoke(); Done @{ ok = $true; how = '点了' } }
          if ($e.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$pat)) { $pat.Select(); Done @{ ok = $true; how = '选中了' } }
          if ($e.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pat)) { $pat.Toggle(); Done @{ ok = $true; how = '勾选状态翻过来了' } }
          if ($e.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$pat)) { $pat.Expand(); Done @{ ok = $true; how = '展开了' } }
          # 这一项不接受上面几种办法：用鼠标真的点一下它的正中，点完把鼠标放回原处
          $r = $e.Current.BoundingRectangle; [long]$was = 0; [void][Y.M]::GetCursorPos([ref]$was)
          [void][Y.M]::SetCursorPos([int]($r.Left + $r.Width / 2), [int]($r.Top + $r.Height / 2))
          [Y.M]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [Y.M]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60
          [void][Y.M]::SetCursorPos([int]($was -band 0xFFFFFFFF), [int]($was -shr 32))
          Done @{ ok = $true; how = '用鼠标点了一下' }
        }
        'type' {
          # 一个键一个键地打进去：给那些不让直接填字的地方用
          $e.SetFocus(); Start-Sleep -Milliseconds 150
          Add-Type -AssemblyName System.Windows.Forms
          $s = [regex]::Replace([string]$q.text, '[+^%~(){}\[\]]', '{$0}') -replace "`r?`n", '{ENTER}'
          [System.Windows.Forms.SendKeys]::SendWait($s); Done @{ ok = $true; how = '打进去了' }
        }
        'key' {
          # 只认这张表里的键。关窗口、切程序、系统键不在里面
          $keys = @{ 'enter' = '{ENTER}'; 'tab' = '{TAB}'; 'esc' = '{ESC}'; 'backspace' = '{BACKSPACE}'; 'delete' = '{DELETE}'; 'up' = '{UP}'; 'down' = '{DOWN}'; 'left' = '{LEFT}'; 'right' = '{RIGHT}'
            'home' = '{HOME}'; 'end' = '{END}'; 'pageup' = '{PGUP}'; 'pagedown' = '{PGDN}'; 'f5' = '{F5}'; 'ctrl+home' = '^{HOME}'; 'ctrl+end' = '^{END}'; 'space' = ' '; 'ctrl+l' = '^l'; 'ctrl+t' = '^t'; 'ctrl+w' = '^w'; 'ctrl+tab' = '^{TAB}'; 'ctrl+r' = '^r'; 'alt+left' = '%{LEFT}'; 'alt+right' = '%{RIGHT}'; 'ctrl+a' = '^a'; 'ctrl+c' = '^c'; 'ctrl+v' = '^v'; 'ctrl+x' = '^x'; 'ctrl+z' = '^z'; 'ctrl+y' = '^y'; 'ctrl+s' = '^s'; 'ctrl+f' = '^f' }
          $k = "$($q.text)".ToLower().Trim(); if (-not $keys.ContainsKey($k)) { Fail "这个键不在能按的那张表里：$k。表里有：$(($keys.Keys | Sort-Object) -join '、')" }
          $e.SetFocus(); Start-Sleep -Milliseconds 150
          Add-Type -AssemblyName System.Windows.Forms
          [System.Windows.Forms.SendKeys]::SendWait($keys[$k]); Done @{ ok = $true; how = "按了 $k" }
        }
        'menu' {
          # 在它上面点右键，把菜单叫出来。菜单里有什么，要再看一遍
          $r = $e.Current.BoundingRectangle; [long]$was = 0; [void][Y.M]::GetCursorPos([ref]$was)
          [void][Y.M]::SetCursorPos([int]($r.Left + $r.Width / 2), [int]($r.Top + $r.Height / 2))
          [Y.M]::mouse_event(8, 0, 0, 0, [UIntPtr]::Zero); [Y.M]::mouse_event(16, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 250
          [void][Y.M]::SetCursorPos([int]($was -band 0xFFFFFFFF), [int]($was -shr 32))
          Done @{ ok = $true; how = '右键菜单叫出来了' }
        }
        'drag' {
          if ($q.to -lt 0 -or $q.to -ge $list.Count) { Fail '要拖到的那一项不在了' }
          $b = $list[[int]$q.to]; $bn = "$($b.Current.Name)".Trim(); if ($bn.Length -gt 60) { $bn = $bn.Substring(0, 60) }
          if ($bn -ne "$($q.to_name)") { Fail '界面变了，要拖到的那一项现在是别的东西' }
          $ra = $e.Current.BoundingRectangle; $rb = $b.Current.BoundingRectangle; [long]$was = 0; [void][Y.M]::GetCursorPos([ref]$was)
          $ax = $ra.Left + $ra.Width / 2; $ay = $ra.Top + $ra.Height / 2; $bx = $rb.Left + $rb.Width / 2; $by = $rb.Top + $rb.Height / 2
          [void][Y.M]::SetCursorPos([int]$ax, [int]$ay); Start-Sleep -Milliseconds 80; [Y.M]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
          1..12 | ForEach-Object { [void][Y.M]::SetCursorPos([int]($ax + ($bx - $ax) * $_ / 12), [int]($ay + ($by - $ay) * $_ / 12)); Start-Sleep -Milliseconds 25 }
          [Y.M]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 80
          [void][Y.M]::SetCursorPos([int]($was -band 0xFFFFFFFF), [int]($was -shr 32))
          Done @{ ok = $true; how = '拖过去了' }
        }
        default { Fail "不认识的动作：$($q.action)" }
      }
    }
    default { Fail "不认识的操作：$($q.op)" }
  }
} catch { Fail "$($_.Exception.Message)" }
