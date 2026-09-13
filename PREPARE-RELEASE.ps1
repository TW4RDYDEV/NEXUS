$ErrorActionPreference = 'Stop'

Write-Host 'Preparing NEXUS v1.0.0 release metadata...' -ForegroundColor Cyan

if (Test-Path '.\LICENSE-PENDING.md') {
    Remove-Item '.\LICENSE-PENDING.md' -Force
    Write-Host '[OK] Removed LICENSE-PENDING.md' -ForegroundColor Green
}

if (-not (Test-Path '.\LICENSE')) {
    throw 'LICENSE is missing.'
}

npm run release:check

Write-Host ''
Write-Host 'NEXUS release metadata is ready.' -ForegroundColor Green
Write-Host 'Next: run the full checks, then npm run release:build.'
