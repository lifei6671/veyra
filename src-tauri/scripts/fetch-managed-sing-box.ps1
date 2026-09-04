[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$archiveUrl = 'https://github.com/SagerNet/sing-box/releases/download/v1.14.0/sing-box-1.14.0-windows-amd64.zip'
$archiveHash = '3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6'
$assetRoot = Split-Path -Parent $PSScriptRoot
$cacheRoot = Join-Path $assetRoot 'binaries'
$archivePath = Join-Path $cacheRoot 'sing-box-1.14.0-windows-amd64.zip'
$extractRoot = Join-Path $cacheRoot 'sing-box-1.14.0-windows-amd64'

$expectedFiles = [ordered]@{
    'sing-box.exe' = 'aad0ede010eafa7b277e520464f3a66fde820103d737eff739f40f3cc9451dcc'
    'libcronet.dll' = 'eee741046f0a3975124bae349aeac237aa306f3cc4de59ff5de070e74dbfdaeb'
    'LICENSE' = 'bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c'
}

New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
if (-not (Test-Path -LiteralPath $archivePath)) {
    Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath
}

$actualArchiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash.ToLowerInvariant()
if ($actualArchiveHash -ne $archiveHash) {
    throw 'Managed sing-box archive SHA-256 verification failed.'
}

if (-not (Test-Path -LiteralPath $extractRoot)) {
    Expand-Archive -LiteralPath $archivePath -DestinationPath $cacheRoot
}

$actualFiles = @(Get-ChildItem -LiteralPath $extractRoot -File -Recurse | ForEach-Object {
    $_.FullName.Substring($extractRoot.Length + 1)
})
$actualFileSet = @($actualFiles | Sort-Object) -join "`n"
$expectedFileSet = @($expectedFiles.Keys | Sort-Object) -join "`n"
if ($actualFileSet -ne $expectedFileSet) {
    throw 'Managed sing-box archive member verification failed.'
}

foreach ($entry in $expectedFiles.GetEnumerator()) {
    $path = Join-Path $extractRoot $entry.Key
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    if ($actualHash -ne $entry.Value) {
        throw "Managed sing-box content verification failed for $($entry.Key)."
    }
}

$versionOutput = & (Join-Path $extractRoot 'sing-box.exe') version | Out-String
if ($LASTEXITCODE -ne 0 -or $versionOutput -notmatch '(?m)^sing-box version 1\.14\.0(?:\s|$)') {
    throw 'Managed sing-box version verification failed.'
}
