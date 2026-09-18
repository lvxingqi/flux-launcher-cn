#Requires -Version 7
<#
.SYNOPSIS
    Validates that free-form workflow inputs are not interpolated into PowerShell run blocks.

.DESCRIPTION
    GitHub expands ${{ inputs.NAME }} before the shell ever sees the script, so a
    string input containing a backtick, a quote or a dollar sign changes how
    PowerShell parses the step. A release run failed exactly this way: the
    hand-written release notes contained backticks and the notes validation step
    aborted with a ParserError before the release was created.

    Every workflow_dispatch input whose type is string (or omitted, which
    defaults to string) must therefore reach the script through env: and be read
    as $env:NAME. Boolean, choice and number inputs remain safe to interpolate
    because their values cannot contain shell syntax.

    The check is text based: it collects the workflow_dispatch input types and
    then scans every run block for interpolations, so no YAML module is needed.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$WorkflowDirectory
)

$files = @(Get-ChildItem $WorkflowDirectory -Filter '*.yml' -File)
if ($files.Count -eq 0) {
    throw "未在 $WorkflowDirectory 找到 workflow 文件。"
}

function Get-InputIndent([string]$Line) {
    if ($Line -match '^(\s*)') {
        return $Matches[1].Length
    }
    return 0
}

function Get-DispatchInputType {
    param([string[]]$Lines)

    $inputs = @{}
    $inInputs = $false
    $inputsIndent = 0
    $currentName = $null
    $currentIndent = 0

    for ($index = 0; $index -lt $Lines.Count; $index++) {
        $line = $Lines[$index]
        if ($line.Trim() -eq '' -or $line.TrimStart().StartsWith('#')) {
            continue
        }

        if (-not $inInputs) {
            if ($line -match '^\s*inputs:\s*$') {
                $inInputs = $true
                $inputsIndent = Get-InputIndent $line
            }
            continue
        }

        $indent = Get-InputIndent $line
        if ($indent -le $inputsIndent) {
            break
        }

        if ($line -match '^\s*([A-Za-z_][A-Za-z0-9_-]*):\s*$') {
            $currentName = $Matches[1]
            $currentIndent = $indent
            if (-not $inputs.ContainsKey($currentName)) {
                $inputs[$currentName] = 'string'
            }
            continue
        }

        if ($null -ne $currentName -and $indent -gt $currentIndent -and $line -match '^\s*type:\s*(\S+)\s*$') {
            $inputs[$currentName] = $Matches[1].ToLowerInvariant()
        }
    }

    return $inputs
}

function Get-RunBlock {
    param([string[]]$Lines)

    $blocks = [System.Collections.Generic.List[object]]::new()

    for ($index = 0; $index -lt $Lines.Count; $index++) {
        $line = $Lines[$index]
        if ($line -notmatch '^(\s*)(?:- )?run:\s*(.*)$') {
            continue
        }

        $runIndent = $Matches[1].Length
        $rest = $Matches[2].Trim()

        if ($rest -match '^\|' -or $rest -match '^>') {
            $body = [System.Collections.Generic.List[string]]::new()
            for ($scan = $index + 1; $scan -lt $Lines.Count; $scan++) {
                $candidate = $Lines[$scan]
                if ($candidate.Trim() -ne '') {
                    if ((Get-InputIndent $candidate) -le $runIndent) {
                        break
                    }
                }
                $body.Add($candidate)
            }
            $blocks.Add([pscustomobject]@{ Line = $index + 1; Body = $body -join "`n" })
        }
        else {
            $blocks.Add([pscustomobject]@{ Line = $index + 1; Body = $rest })
        }
    }

    return $blocks
}

$problems = [System.Collections.Generic.List[string]]::new()
$checkedRuns = 0

foreach ($file in $files) {
    $lines = @(Get-Content $file.FullName)
    $inputTypes = Get-DispatchInputType -Lines $lines

    foreach ($block in (Get-RunBlock -Lines $lines)) {
        $checkedRuns++

        foreach ($match in [regex]::Matches($block.Body, '\$\{\{\s*inputs\.([A-Za-z_][A-Za-z0-9_-]*)\s*\}\}')) {
            $name = $match.Groups[1].Value
            $type = if ($inputTypes.ContainsKey($name)) { $inputTypes[$name] } else { $null }

            if ($null -eq $type) {
                $problems.Add("$($file.Name):$($block.Line): run 块引用了未声明的输入 '$name'。")
                continue
            }

            if ($type -eq 'string') {
                $problems.Add("$($file.Name):$($block.Line): 字符串输入 '$name' 被直接插值进 PowerShell run 块；请改用 env: 并以 `$env:$name 读取，避免反引号、引号或 `$ 改变解析结果。")
            }
        }
    }
}

if ($problems.Count -gt 0) {
    $problems | ForEach-Object { Write-Host $_ }
    throw "workflow 输入插值校验失败，共 $($problems.Count) 处。"
}

Write-Host "workflow 输入插值校验通过：$($files.Count) 个 workflow，检查 run 块 $checkedRuns 处，字符串输入均通过 env 传递。"