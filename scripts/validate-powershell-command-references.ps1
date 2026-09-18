#Requires -Version 7
<#
.SYNOPSIS
    Validates that every command referenced by a repository PowerShell script resolves.

.DESCRIPTION
    Commit 3b52cee renamed Update-Smoke helper functions to satisfy the PSSA
    PSUseSingularNouns rule but left one call site using the old plural name, so
    update-smoke.ps1 only failed on the first Windows release run that ever
    executed it. Scripts under scripts/ never dot-source each other, therefore a
    called command must resolve either to a function defined in the same file or
    to a real command available in the environment. Anything else is a latent
    runtime failure.

    Names without a hyphen (git, cargo, gh, ...) are skipped because they are
    external tools resolved through PATH; names containing a path separator or a
    file extension are skipped as well.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$ScriptsDirectory
)

$files = @(Get-ChildItem $ScriptsDirectory -Filter '*.ps1' -File)
if ($files.Count -eq 0) {
    throw "未在 $ScriptsDirectory 找到 PowerShell 脚本。"
}

$parsed = @{}
$definedAnywhere = [System.Collections.Generic.HashSet[string]]::new()

foreach ($file in $files) {
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($file.FullName, [ref]$tokens, [ref]$errors)

    if ($errors.Count -gt 0) {
        throw "$($file.Name) 存在语法错误，无法校验命令引用：$($errors[0].Message)"
    }

    $functions = @(
        $ast.FindAll({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] }, $true) |
            ForEach-Object { $_.Name }
    )

    $parsed[$file.Name] = @{
        Ast = $ast
        Functions = $functions
    }

    foreach ($function in $functions) {
        [void]$definedAnywhere.Add($function)
    }
}

$problems = [System.Collections.Generic.List[string]]::new()

foreach ($file in $files) {
    $entry = $parsed[$file.Name]
    $commands = $entry.Ast.FindAll({ param($node) $node -is [System.Management.Automation.Language.CommandAst] }, $true)

    foreach ($command in $commands) {
        $element = $command.CommandElements[0]

        if ($element -isnot [System.Management.Automation.Language.StringConstantExpressionAst]) {
            continue
        }

        $name = $element.Value

        if ($name -match '[\\/]' -or $name -match '\.(ps1|exe|cmd|bat|py|iss|json|txt|log|md)$') {
            continue
        }

        # 无连字符的名称是经 PATH 解析的外部工具（git、cargo、gh 等）。
        if ($name -notmatch '-') {
            continue
        }

        if ($entry.Functions -contains $name) {
            continue
        }

        if (Get-Command -Name $name -ErrorAction SilentlyContinue) {
            continue
        }

        if ($definedAnywhere.Contains($name)) {
            $problems.Add("$($file.Name):$($command.Extent.StartLineNumber): 调用了其他脚本定义的 '$name'，但本仓库脚本之间没有 dot-source，运行时会失败。")
        } else {
            $problems.Add("$($file.Name):$($command.Extent.StartLineNumber): 无法解析命令 '$name'（既未在本文件定义，也不是可用命令）。")
        }
    }
}

if ($problems.Count -gt 0) {
    $problems | ForEach-Object { Write-Host $_ }
    throw "PowerShell 命令引用校验失败，共 $($problems.Count) 处。"
}

Write-Host "PowerShell 命令引用校验通过：$($files.Count) 个脚本，定义函数 $($definedAnywhere.Count) 个。"
