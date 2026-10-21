param(
    [ValidateSet('Launch', 'Doctor', 'Drive', 'Evidence', 'Cleanup')]
    [string]$Action = 'Drive',
    [string]$Evidence = 'evidence/memoir'
)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '../..')
$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
switch ($Action) {
    Launch { cargo +1.99.1 build --locked -p optd-datafusion-bridge --example memoir }
    Doctor {
        cargo +1.99.1 --version
        if (-not (Test-Path 'target/debug/examples/memoir.exe') -and -not (Test-Path 'target/debug/examples/memoir')) { throw 'Run Launch first' }
        git rev-parse HEAD
    }
    Drive { cargo +1.99.1 run --locked -p optd-datafusion-bridge --example memoir -- $Evidence }
    Evidence {
        foreach ($query in @('skewed_join', 'aggregate', 'empty')) {
            foreach ($suffix in @('sql', 'results.txt', 'plan.txt', 'memo.txt')) {
                if (-not (Test-Path (Join-Path $Evidence "$query.$suffix"))) { throw "Missing $query.$suffix" }
            }
        }
        Get-Content (Join-Path $Evidence 'summary.txt')
    }
    Cleanup { Write-Output 'Short-lived in-memory process; no persistent fixture or background process. Evidence retained.' }
}
if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "Command failed: $LASTEXITCODE" }
