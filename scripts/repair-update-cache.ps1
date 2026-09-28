param(
    [string]$CacheDirectory = (Join-Path $env:LOCALAPPDATA 'com.d2rhub.app\downloads')
)
$ErrorActionPreference = 'Stop'
if (Get-Process -Name d2rhub -ErrorAction SilentlyContinue) {
    throw 'Exit D2RHub completely (including its tray icon), then run this tool again.'
}
$cache = Join-Path $CacheDirectory 'software-v2.json'
if (-not (Test-Path -LiteralPath $cache)) {
    Write-Output 'No software update cache found. Nothing changed.'
    exit 0
}
foreach ($path in @($CacheDirectory, $cache)) {
    if ((Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw 'Linked cache paths are not supported. Nothing changed.'
    }
}
if ((Get-Item -LiteralPath $cache).Length -gt 262144) {
    throw 'Unexpected cache size. Nothing changed.'
}
$manifest = Get-Content -LiteralPath $cache -Raw -Encoding UTF8 | ConvertFrom-Json
$assets = @($manifest.assets)
if ($manifest.schema -ne 2 -or $manifest.kind -ne 'software' -or
    $manifest.product -ne 'D2RHub' -or $manifest.platform -ne 'windows-x86_64' -or
    $assets.Count -ne 1 -or $assets[0].id -ne 'hub') {
    throw 'Unrecognized cache format. Nothing changed.'
}
if ($assets[0].version -ne '0.99.106') {
    Write-Output 'The cache does not contain the mistyped version. Nothing changed.'
    exit 0
}
if ($assets[0].sha256 -ne '78e51bb765614c8ad0b92cdb49dd2f9242c828641a5aab9de561c448a55c4332' -or
    $assets[0].size -ne 7737897) {
    throw 'The cached artifact is not the known 0.99.106 release. Nothing changed.'
}
$backup = "$cache.before-version-repair-$([Guid]::NewGuid().ToString('N')).bak"
Move-Item -LiteralPath $cache -Destination $backup
Write-Output "Update cache backed up to: $backup"
Write-Output 'Restart D2RHub 0.9.104/0.9.105 and check for updates again.'
Write-Output 'If the installed app itself is 0.99.106, manually install the current release:'
Write-Output 'https://github.com/gjy991229/D2rHub/releases/latest'
Write-Output 'Accounts, settings, downloaded installers and Mods were not changed.'
