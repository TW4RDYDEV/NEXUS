param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[0-9a-fA-F]{40}$')][string]$CertificateThumbprint,
    [Parameter(Mandatory=$true)][ValidatePattern('^https://')][string]$TimestampUrl,
    [string]$SignTool = 'signtool.exe'
)
$ErrorActionPreference = 'Stop'
$releaseRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\release\windows-x64'))
$releaseExe = Join-Path $releaseRoot 'NEXUS.exe'
if (-not (Test-Path -LiteralPath $releaseExe -PathType Leaf)) { throw 'Build and stage the Windows release first.' }
$installer = Get-ChildItem -LiteralPath $releaseRoot -Filter 'NEXUS-Setup-*.exe' -File -ErrorAction SilentlyContinue | Select-Object -First 1
$targets = @($releaseExe)
if ($installer) { $targets += $installer.FullName }
$certificate = Get-Item -LiteralPath ('Cert:\CurrentUser\My\' + $CertificateThumbprint)
if (-not $certificate.HasPrivateKey -or $certificate.NotAfter -lt (Get-Date)) { throw 'A valid code-signing certificate with its private key is required.' }
if (-not ($certificate.EnhancedKeyUsageList.ObjectId.Value -contains '1.3.6.1.5.5.7.3.3')) { throw 'The certificate does not permit code signing.' }
foreach ($target in $targets) {
    & $SignTool sign /sha1 $CertificateThumbprint /fd SHA256 /tr $TimestampUrl /td SHA256 $target
    if ($LASTEXITCODE -ne 0) { throw "Signing failed: $target" }
    & $SignTool verify /pa /all /v $target
    if ($LASTEXITCODE -ne 0) { throw "Signature verification failed: $target" }
    Get-AuthenticodeSignature -LiteralPath $target | Select-Object Path,Status,SignerCertificate
}
Write-Output 'Regenerate SHA256SUMS.txt and release archive hashes after signing.'
