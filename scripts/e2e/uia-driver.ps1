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
    throw 'control has no invoke, toggle or selection pattern'
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
