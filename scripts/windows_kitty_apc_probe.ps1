param(
    [Parameter(Mandatory = $true)]
    [string]$ProbeExePath,

    [string]$BundlePath,

    [switch]$UseSystemConpty
)

$ErrorActionPreference = "Stop"
$probe = (Resolve-Path $ProbeExePath).Path
$probeDir = Split-Path -Parent $probe
$sourceBundle = if ([string]::IsNullOrWhiteSpace($BundlePath)) {
    Join-Path (Split-Path -Parent $probeDir) "conpty"
} else {
    (Resolve-Path $BundlePath).Path
}
$probeBundle = Join-Path $probeDir "conpty"
if (-not $UseSystemConpty -and -not (Test-Path (Join-Path $sourceBundle "conpty.dll"))) {
    throw "missing bundled ConPTY: $sourceBundle"
}

$oldConptyMode = $env:HERDR_WINDOWS_CONPTY
try {
    if ($UseSystemConpty) {
        $env:HERDR_WINDOWS_CONPTY = "system"
    } else {
        if (Test-Path $probeBundle) {
            Remove-Item -LiteralPath $probeBundle -Recurse -Force
        }
        Copy-Item -LiteralPath $sourceBundle -Destination $probeBundle -Recurse
        Remove-Item Env:HERDR_WINDOWS_CONPTY -ErrorAction SilentlyContinue
    }

    $output = & $probe 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Kitty APC passthrough probe failed with exit code $LASTEXITCODE`: $($output -join "`n")"
    }
    if (($output -join "`n") -notmatch "Kitty APC passthrough: OK") {
        throw "Kitty APC passthrough probe did not report success: $($output -join "`n")"
    }
    Write-Host ($output -join "`n")
} finally {
    if (-not $UseSystemConpty) {
        Remove-Item -LiteralPath $probeBundle -Recurse -Force -ErrorAction SilentlyContinue
    }
    if ($null -eq $oldConptyMode) {
        Remove-Item Env:HERDR_WINDOWS_CONPTY -ErrorAction SilentlyContinue
    } else {
        $env:HERDR_WINDOWS_CONPTY = $oldConptyMode
    }
}
