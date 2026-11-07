# Drives the running MedScale window through Windows UI Automation, the interface
# assistive technology uses. It needs no debugging port or test feature, so the
# app under test is byte-for-byte the installed release. One JSON command per line
# in, one JSON reply per line out. Replies never carry page text to any log: the
# caller decides what it may print.
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$ErrorActionPreference = 'Stop'
$ae = [System.Windows.Automation.AutomationElement]
$scope = [System.Windows.Automation.TreeScope]
$cond = [System.Windows.Automation.Condition]::TrueCondition
$script:window = $null

function Find-Window([int]$ProcessId) {
    $byPid = New-Object System.Windows.Automation.PropertyCondition($ae::ProcessIdProperty, $ProcessId)
    return $ae::RootElement.FindFirst($scope::Children, $byPid)
}

function Find-Control($c) {
    if ($c.id) {
        $by = New-Object System.Windows.Automation.PropertyCondition($ae::AutomationIdProperty, [string]$c.id)
    } else {
        $by = New-Object System.Windows.Automation.PropertyCondition($ae::NameProperty, [string]$c.name)
    }
    foreach ($e in $script:window.FindAll($scope::Descendants, $by)) {
        if (-not $c.kind) { return $e }
        if ($e.Current.ControlType.ProgrammaticName -eq "ControlType.$($c.kind)") { return $e }
    }
    # An accessible name can carry more than the visible label.
    if ($c.name -and $c.kind) {
        $kind = [System.Windows.Automation.ControlType]::$($c.kind)
        $byKind = New-Object System.Windows.Automation.PropertyCondition($ae::ControlTypeProperty, $kind)
        foreach ($e in $script:window.FindAll($scope::Descendants, $byKind)) {
            if ($e.Current.Name.Contains([string]$c.name)) { return $e }
        }
    }
    return $null
}

function Invoke-Control($e) {
    $p = $null
    if ($e.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$p)) { $p.Invoke(); return }
    if ($e.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$p)) { $p.Toggle(); return }
    if ($e.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$p)) { $p.Select(); return }
    if ($e.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$p)) {
        # A button with aria-expanded, such as a sidebar group, offers only this.
        if ($p.Current.ExpandCollapseState -eq [System.Windows.Automation.ExpandCollapseState]::Collapsed) { $p.Expand() } else { $p.Collapse() }
        return
    }
    throw 'control has no invoke, toggle, selection or expand pattern'
}

while ($null -ne ($line = [Console]::In.ReadLine())) {
    $reply = @{ ok = $true }
    try {
        $c = $line | ConvertFrom-Json
        switch ($c.op) {
            'attach' {
                $script:window = Find-Window $c.pid
                $reply.found = $null -ne $script:window
            }
            'exists' {
                $e = Find-Control $c
                $reply.found = $null -ne $e
                $reply.enabled = ($null -ne $e) -and $e.Current.IsEnabled
            }
            'invoke' {
                $e = Find-Control $c
                if ($null -eq $e) { throw 'not found' }
                if (-not $e.Current.IsEnabled) { throw 'disabled' }
                Invoke-Control $e
            }
            'set' {
                $e = Find-Control $c
                if ($null -eq $e) { throw 'not found' }
                $p = $null
                if (-not $e.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$p)) { throw 'no value pattern' }
                $p.SetValue([string]$c.value)
            }
            'checked' {
                $e = Find-Control $c
                if ($null -eq $e) { throw 'not found' }
                $p = $null
                if (-not $e.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$p)) { throw 'no toggle pattern' }
                $reply.checked = $p.Current.ToggleState -eq [System.Windows.Automation.ToggleState]::On
            }
            'selected' {
                $e = Find-Control $c
                if ($null -eq $e) { throw 'not found' }
                $p = $null
                if (-not $e.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$p)) { throw 'no selection pattern' }
                $reply.selected = $p.Current.IsSelected
            }
            'unnamed' {
                # Focusable controls with no accessible name. Counts only.
                $by = New-Object System.Windows.Automation.PropertyCondition($ae::IsKeyboardFocusableProperty, $true)
                $counts = @{}
                foreach ($e in $script:window.FindAll($scope::Descendants, $by)) {
                    if (-not $e.Current.Name) {
                        $t = $e.Current.ControlType.ProgrammaticName
                        $counts[$t] = 1 + [int]$counts[$t]
                    }
                }
                $reply.unnamed = $counts
            }
            'resize' {
                $p = $null
                if (-not $script:window.TryGetCurrentPattern([System.Windows.Automation.TransformPattern]::Pattern, [ref]$p)) { throw 'no transform pattern' }
                $p.Resize([double]$c.width, [double]$c.height)
                $r = $script:window.Current.BoundingRectangle
                $reply.width = $r.Width; $reply.height = $r.Height
            }
            'clipped' {
                # Named controls and text that extend past the window's right edge.
                $w = $script:window.Current.BoundingRectangle
                $names = New-Object System.Collections.Generic.List[string]
                foreach ($e in $script:window.FindAll($scope::Descendants, $cond)) {
                    $r = $e.Current.BoundingRectangle
                    if ($r.IsEmpty -or $e.Current.IsOffscreen) { continue }
                    if ($r.Right -gt ($w.Right + 1) -and $e.Current.Name) { $names.Add($e.Current.Name) }
                }
                $reply.width = $w.Width; $reply.names = $names.ToArray()
            }
            'screenshot' {
                # The window as a person sees it. Callers use this only on synthetic screens.
                Add-Type -AssemblyName System.Drawing
                $r = $script:window.Current.BoundingRectangle
                $bmp = New-Object System.Drawing.Bitmap([int]$r.Width, [int]$r.Height)
                $g = [System.Drawing.Graphics]::FromImage($bmp)
                $g.CopyFromScreen([int]$r.X, [int]$r.Y, 0, 0, $bmp.Size)
                $bmp.Save([string]$c.path, [System.Drawing.Imaging.ImageFormat]::Png)
                $g.Dispose(); $bmp.Dispose()
            }
            'file-icon' {
                Add-Type -AssemblyName System.Drawing
                $icon = [System.Drawing.Icon]::ExtractAssociatedIcon([string]$c.file)
                $bmp = $icon.ToBitmap()
                $bmp.Save([string]$c.path, [System.Drawing.Imaging.ImageFormat]::Png)
                $bmp.Dispose(); $icon.Dispose()
            }
            'shape' {
                # Counts by control type only, so it's always safe to print.
                $counts = @{}
                foreach ($e in $script:window.FindAll($scope::Descendants, $cond)) {
                    $t = $e.Current.ControlType.ProgrammaticName
                    $counts[$t] = 1 + [int]$counts[$t]
                }
                $reply.shape = $counts
            }
            'wake' {
                # Chromium can drop its accessibility tree; focus and a fresh
                # lookup of the window ask it to build one again.
                $script:window = Find-Window $c.pid
                if ($null -ne $script:window) { $script:window.SetFocus() }
                $reply.found = $null -ne $script:window
            }
            'text' {
                $names = New-Object System.Collections.Generic.List[string]
                foreach ($e in $script:window.FindAll($scope::Descendants, $cond)) {
                    $n = $e.Current.Name
                    if ($n) { $names.Add($n) }
                }
                $reply.names = $names.ToArray()
            }
            default { throw 'unknown op' }
        }
    } catch {
        $reply = @{ ok = $false; error = $_.Exception.Message }
    }
    [Console]::Out.WriteLine(($reply | ConvertTo-Json -Compress -Depth 3))
    [Console]::Out.Flush()
}
