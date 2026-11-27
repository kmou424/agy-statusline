Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$installer = Join-Path $PSScriptRoot 'install.ps1'
$tmpDir = Join-Path ([IO.Path]::GetTempPath()) ('test-agy-statusline-' + [Guid]::NewGuid())
$fixtureEnv = @{
    LOCALAPPDATA = (Join-Path $tmpDir 'local')
    AGY_STATUSLINE_HOME = (Join-Path $tmpDir 'home')
    AGY_STATUSLINE_RELEASE_BASE_URL = ([Uri](Join-Path $tmpDir 'releases')).AbsoluteUri
    AGY_STATUSLINE_LATEST_TAG = 'v0.1.2'
    AGY_STATUSLINE_ARCH = 'AMD64'
}
$savedEnv = @{}
foreach ($key in $fixtureEnv.Keys) {
    $savedEnv[$key] = [Environment]::GetEnvironmentVariable($key)
}
$releases = Join-Path $tmpDir 'releases'
$payloadDir = Join-Path $tmpDir 'payload'
$target = 'x86_64-pc-windows-msvc'
$settingsDir = Join-Path $fixtureEnv.AGY_STATUSLINE_HOME '.gemini/antigravity-cli'
$settings = Join-Path $settingsDir 'settings.json'
$binary = Join-Path $fixtureEnv.LOCALAPPDATA 'Programs/agy-statusline/agy-statusline.exe'

function New-MockRelease($Tag, $Payload) {
    $dir = Join-Path $releases $Tag
    [void][IO.Directory]::CreateDirectory($dir)
    $archive = "agy-statusline-$Tag-$target.zip"
    $zip = Join-Path $dir $archive
    [IO.File]::WriteAllText((Join-Path $payloadDir 'agy-statusline.exe'), $Payload)
    [IO.File]::WriteAllText((Join-Path $payloadDir 'README.md'), 'Mock release README')
    Compress-Archive -Path (Join-Path $payloadDir '*') -DestinationPath $zip
    $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText((Join-Path $dir 'SHA256SUMS'), "$hash  $archive`n")
}

function Assert-Settings($Command, [bool]$Stack) {
    $data = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($settings))
    $line = $data.statusLine
    if ($data.other.theme -cne 'test' -or $data.other.values.Count -ne 2 -or
        $data.other.values[0] -ne 3 -or $data.other.values[1] -ne $true -or $line.padding -ne 2) {
        throw 'Unrelated settings changed.'
    }
    if ($line.type -cne 'command' -or $line.command -cne $Command -or $line.enabled -ne $true) {
        throw 'Unexpected statusLine configuration.'
    }
    $stacked = $line.PSObject.Properties['stack_with_default']
    if ($Stack -and ($null -eq $stacked -or $stacked.Value -ne $true)) { throw 'Expected stacking.' }
    if (!$Stack -and $null -ne $stacked) { throw 'Expected stack_with_default to be omitted.' }
}

function Assert-Title($Command) {
    $data = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($settings))
    $title = $data.title
    if ($title.type -cne 'command' -or $title.command -cne $Command -or
        $title.enabled -ne $true -or $title.prefix -cne 'keep') {
        throw 'Unexpected title configuration.'
    }
}

function Get-TitleJson {
    $data = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($settings))
    ConvertTo-Json -InputObject $data.title -Depth 100 -Compress
}

try {
    foreach ($key in $fixtureEnv.Keys) {
        [Environment]::SetEnvironmentVariable($key, $fixtureEnv[$key])
    }
    [void][IO.Directory]::CreateDirectory($payloadDir)
    [void][IO.Directory]::CreateDirectory($settingsDir)
    [IO.File]::WriteAllText($settings, '{"other":{"theme":"test","values":[3,true]},"statusLine":{"enabled":false,"padding":2,"stack_with_default":true},"title":{"prefix":"keep","enabled":false}}')
    New-MockRelease 'v0.1.2' 'stable'
    New-MockRelease 'canary' 'canary'
    New-MockRelease 'v0.1.1' 'legacy'

    Invoke-Expression ([IO.File]::ReadAllText($installer))
    if ([IO.File]::ReadAllText($binary) -cne 'stable') { throw 'Expected stable binary.' }
    $command = $binary
    if ($command -match '[\s&()^%!;,=]') { $command = '"' + $command + '"' }
    Assert-Settings $command $true
    Assert-Title "$command title"

    $remoteScript = [scriptblock]::Create([IO.File]::ReadAllText($installer))
    & $remoteScript -Canary
    if ([IO.File]::ReadAllText($binary) -cne 'canary') { throw 'Expected canary binary.' }
    $untouched = [IO.File]::ReadAllText($settings)
    $unconfigured = Join-Path $tmpDir 'unconfigured'
    $unconfiguredBinary = Join-Path $unconfigured 'agy-statusline.exe'
    & $installer -Version 'v0.1.2' -InstallDir $unconfigured -NoConfigure
    if ([IO.File]::ReadAllText($unconfiguredBinary) -cne 'stable') { throw 'Expected the requested stable tag.' }
    if ([IO.File]::ReadAllText($settings) -cne $untouched) { throw 'Expected -NoConfigure to preserve settings.' }
    & $installer --canary --no-configure -InstallDir $unconfigured
    if ([IO.File]::ReadAllText($unconfiguredBinary) -cne 'canary') {
        throw 'Expected --canary to select canary.'
    }
    if ([IO.File]::ReadAllText($settings) -cne $untouched) { throw 'Expected --no-configure to preserve settings.' }
    $helpText = & $installer --help
    if ($helpText -notmatch '^Usage:.*\[-NoTitle\]') { throw 'Expected --help to list -NoTitle.' }

    $legacy = Join-Path $tmpDir 'legacy tools'
    $legacyBinary = Join-Path $legacy 'agy-statusline.exe'
    $titleBeforeLegacy = Get-TitleJson
    $stderr = [Console]::Error
    $legacyError = [IO.StringWriter]::new()
    try {
        [Console]::SetError($legacyError)
        & $installer -Version 'v0.1.1' -InstallDir $legacy
    } finally {
        [Console]::SetError($stderr)
    }
    if ([IO.File]::ReadAllText($legacyBinary) -cne 'legacy') { throw 'Expected legacy binary.' }
    Assert-Settings ('"' + $legacyBinary + '"') $true
    if ((Get-TitleJson) -cne $titleBeforeLegacy) { throw 'Expected a legacy install to preserve title.' }
    $legacyNotice = 'agy-statusline: Skipping title configuration (v0.1.1 does not include the title subcommand).'
    if ($legacyError.ToString() -cne ($legacyNotice + [Environment]::NewLine)) {
        throw 'Expected a legacy title skip diagnostic on stderr.'
    }

    $custom = Join-Path $tmpDir 'R&D tools'
    & $installer -Version 'v0.1.2' -InstallDir $custom --stack-with-default
    $customBinary = Join-Path $custom 'agy-statusline.exe'
    if ([IO.File]::ReadAllText($customBinary) -cne 'stable') { throw 'Expected custom binary.' }
    $customCommand = '"' + $customBinary + '"'
    Assert-Settings $customCommand $true
    Assert-Title ('"' + $customBinary + '" title')

    $data = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($settings))
    $data.title.command = 'custom-title'
    $data.title.enabled = $false
    [IO.File]::WriteAllText($settings, (ConvertTo-Json -InputObject $data -Depth 100))
    $customTitle = Get-TitleJson

    & $installer -NoTitle
    Assert-Settings $command $true
    if ((Get-TitleJson) -cne $customTitle) { throw 'Expected -NoTitle to preserve title.' }

    & $installer --no-title -InstallDir $custom
    Assert-Settings $customCommand $true
    if ((Get-TitleJson) -cne $customTitle) { throw 'Expected --no-title to preserve title.' }

    $original = [IO.File]::ReadAllText($settings)

    $archive = "agy-statusline-canary-$target.zip"
    $badHash = '0' * 64
    [IO.File]::WriteAllText((Join-Path $releases 'canary/SHA256SUMS'), "$badHash  $archive`n")
    $rejected = $false
    try {
        & $installer -Canary -InstallDir $custom
    } catch {
        if ($_.Exception.Message -notmatch 'SHA-256 mismatch') { throw }
        $rejected = $true
    }
    if (!$rejected) { throw 'Checksum mismatch was accepted.' }
    if ([IO.File]::ReadAllText($customBinary) -cne 'stable') { throw 'Checksum mismatch replaced binary.' }
    if ([IO.File]::ReadAllText($settings) -cne $original) { throw 'Checksum mismatch changed settings.' }

    Write-Output 'Installer fixture tests passed.'
} finally {
    foreach ($key in $savedEnv.Keys) {
        [Environment]::SetEnvironmentVariable($key, $savedEnv[$key])
    }
    if (Test-Path -LiteralPath $tmpDir) { Remove-Item -LiteralPath $tmpDir -Recurse -Force }
}
