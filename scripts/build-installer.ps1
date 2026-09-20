[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    # 纯数字点分版本（Inno 的 AppVersion/VersionInfoVersion 要求，如 0.3.0）。
    [string]$AppVersion,
    # 完整显示版本，可含 SemVer 预发布标识（如 0.3.0-beta.1）；缺省回退为 AppVersion。
    [string]$DisplayVersion,
    [string]$BuildDir = "target/x86_64-pc-windows-msvc/release",
    [string]$OutputDirectory = "artifacts/FluxLauncher-Windows11-x64",
    [string]$InstallDirectory,
    [string]$InnoCompiler
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$buildPath = (Resolve-Path (Join-Path $repoRoot $BuildDir)).Path
$outputPath = Join-Path $repoRoot $OutputDirectory
$portableName = "FluxLauncher-Portable.exe"
$installerName = "FluxLauncherCN-Setup.exe"
$requestedInnoCompiler = $InnoCompiler

if ($InnoCompiler) {
    $innoCandidates = @($InnoCompiler)
} elseif ($InstallDirectory) {
    $innoCandidates = @((Join-Path $InstallDirectory "ISCC.exe"))
} else {
    $innoCandidates = @(
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        "C:\Program Files\Inno Setup 6\ISCC.exe",
        "C:\Users\$env:USERNAME\AppData\Local\Programs\Inno Setup 6\ISCC.exe"
    )
}

$InnoCompiler = $innoCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (-not $InnoCompiler) {
    if ($InstallDirectory) {
        throw "Inno Setup compiler was not found at $InstallDirectory. Expected ISCC.exe in that directory."
    }
    if ($PSBoundParameters.ContainsKey("InnoCompiler")) {
        throw "Inno Setup compiler was not found at $requestedInnoCompiler"
    }
    throw "Inno Setup compiler was not found in standard locations. Specify -InstallDirectory or -InnoCompiler."
}
$launcher = Join-Path $buildPath "flux-launcher.exe"
if (-not (Test-Path $launcher)) {
    throw "Release launcher was not found at $launcher"
}

New-Item -ItemType Directory -Force -Path $outputPath | Out-Null
if (-not $DisplayVersion) {
    $DisplayVersion = $AppVersion
}
& $InnoCompiler "/DAppVersion=$AppVersion" "/DDisplayVersion=$DisplayVersion" "/DBuildDir=$buildPath" (Join-Path $repoRoot "packaging/installer/FluxLauncher.iss")
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compiler exited with code $LASTEXITCODE"
}

$builtInstaller = Join-Path $repoRoot "artifacts/installer/$installerName"
if (-not (Test-Path $builtInstaller)) {
    throw "Expected installer was not created at $builtInstaller"
}

Copy-Item $builtInstaller (Join-Path $outputPath $installerName) -Force
Copy-Item $launcher (Join-Path $outputPath $portableName) -Force
Get-ChildItem $outputPath -File | Get-FileHash -Algorithm SHA256 | Format-Table -AutoSize
