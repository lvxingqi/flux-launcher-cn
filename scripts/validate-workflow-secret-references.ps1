#Requires -Version 7
<#
.SYNOPSIS
    Validates secrets and variables references inside GitHub workflow files.

.DESCRIPTION
    Extracts every secrets.NAME and vars.NAME reference from the workflow YAML
    files and fails when a name is outside the canonical allowlist. This catches
    typos such as a misspelled WINDOWS_SIGNING_CERTIFICATE_BASE64 that would
    otherwise only surface at release time. Whether a canonical name is actually
    configured in the repository is intentionally not a failure: the workflows
    guard the absence themselves (the WinGet submission fails fast without
    WINGET_GITHUB_TOKEN and the release workflow only signs stable builds), so
    the script only reports the configured status.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$WorkflowDirectory,

    # Query the repository for the configured status of each canonical name.
    [switch]$CheckConfigured
)

# 保持与 AGENTS.md 第 20 节一致：新增或改名时必须同步该节。
$canonicalSecrets = @(
    'WINGET_GITHUB_TOKEN',
    'WINDOWS_SIGNING_CERTIFICATE_BASE64',
    'WINDOWS_SIGNING_CERTIFICATE_PASSWORD'
)
$canonicalVariables = @(
    'WINDOWS_TIMESTAMP_URL'
)

$files = @(Get-ChildItem $WorkflowDirectory -Filter '*.yml' -File)
if ($files.Count -eq 0) {
    throw "未在 $WorkflowDirectory 找到 workflow 文件。"
}

$problems = [System.Collections.Generic.List[string]]::new()
$referencedSecrets = [System.Collections.Generic.HashSet[string]]::new()
$referencedVariables = [System.Collections.Generic.HashSet[string]]::new()

foreach ($file in $files) {
    $text = Get-Content $file.FullName -Raw

    foreach ($match in [regex]::Matches($text, 'secrets\.([A-Za-z_][A-Za-z0-9_]*)')) {
        $name = $match.Groups[1].Value
        if ($canonicalSecrets -notcontains $name) {
            $problems.Add("$($file.Name): 未登记的 secret 引用 '$name'（允许清单外或拼写错误）。")
        }
        [void]$referencedSecrets.Add($name)
    }

    foreach ($match in [regex]::Matches($text, 'vars\.([A-Za-z_][A-Za-z0-9_]*)')) {
        $name = $match.Groups[1].Value
        if ($canonicalVariables -notcontains $name) {
            $problems.Add("$($file.Name): 未登记的 variable 引用 '$name'（允许清单外或拼写错误）。")
        }
        [void]$referencedVariables.Add($name)
    }
}

foreach ($name in $canonicalSecrets) {
    if (-not $referencedSecrets.Contains($name)) {
        Write-Host "提示: secret '$name' 在允许清单中但未被任何 workflow 引用。"
    }
}

foreach ($name in $canonicalVariables) {
    if (-not $referencedVariables.Contains($name)) {
        Write-Host "提示: variable '$name' 在允许清单中但未被任何 workflow 引用。"
    }
}

if ($CheckConfigured) {
    # 不能用 gh repo view：本仓库同时存在 origin 与 upstream 远端，gh 会把仓库解析到
    # upstream，从而对错误的仓库查询密钥并以 403 失败。
    $originUrl = git remote get-url origin
    if ($LASTEXITCODE -ne 0) {
        throw "无法读取 git remote origin，请确认在仓库工作区内运行。"
    }

    $repository = $originUrl -replace '^git@[^:]+:', '' -replace '^https?://[^/]+/', '' -replace '\.git$', ''
    Write-Host "校验仓库：$repository"

    $configuredSecrets = @(gh secret list --repo $repository --json name --jq '.[].name')
    $secretsReadable = $LASTEXITCODE -eq 0

    if (-not $secretsReadable) {
        Write-Host "提示: 无法读取仓库密钥清单（缺少 Actions secrets 读取权限或未登录），跳过逐项配置状态。"
    }

    $configuredVariables = @(gh variable list --repo $repository --json name --jq '.[].name')
    $variablesReadable = $LASTEXITCODE -eq 0

    if (-not $variablesReadable) {
        Write-Host "提示: 无法读取仓库变量清单（缺少 Actions variables 读取权限或未登录），跳过逐项配置状态。"
    }

    if ($secretsReadable) {
        foreach ($name in $canonicalSecrets) {
            $state = if ($configuredSecrets -contains $name) { '已配置' } else { '未配置（workflow 运行时自行守卫）' }
            Write-Host "secret '$name': $state"
        }
    }

    if ($variablesReadable) {
        foreach ($name in $canonicalVariables) {
            $state = if ($configuredVariables -contains $name) { '已配置' } else { '未配置（workflow 运行时自行守卫）' }
            Write-Host "variable '$name': $state"
        }
    }
}

if ($problems.Count -gt 0) {
    $problems | ForEach-Object { Write-Host $_ }
    throw "workflow 密钥引用校验失败，共 $($problems.Count) 处。"
}

Write-Host "Workflow 密钥引用校验通过：secrets=$($referencedSecrets.Count) vars=$($referencedVariables.Count)，允许清单 $($canonicalSecrets.Count + $canonicalVariables.Count) 项。"
