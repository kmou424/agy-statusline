param(
    [Alias('-canary')][switch]$Canary,
    [string]$Version = '',
    [string]$InstallDir = '',
    [Alias('-no-configure')][switch]$NoConfigure,
    [Alias('-stack-with-default')][switch]$StackWithDefault,
    [Alias('-help', 'h')][switch]$Help,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$ExtraArgs
)

& {
    Set-StrictMode -Version Latest
    $ErrorActionPreference = 'Stop'
    if ($PSVersionTable.PSVersion.Major -lt 6) {
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    }

    foreach ($arg in $ExtraArgs) {
        switch ($arg) {
            '--canary' { $Canary = $true }
            '--no-configure' { $NoConfigure = $true }
            '--stack-with-default' { $StackWithDefault = $true }
            '--help' { $Help = $true }
            default { throw "Unknown option: $arg (see --help)" }
        }
    }
    if ($Help) {
        Write-Output 'Usage: install.ps1 [-Canary | -Version TAG] [-InstallDir DIR] [-NoConfigure] [-StackWithDefault]'
        return
    }
    if ($Canary -and $Version) { throw 'Choose either -Canary or -Version.' }

    function Get-ReleaseAsset($Url, $Destination) {
        $uri = [Uri]$Url
        if ($uri.IsFile) {
            Copy-Item -LiteralPath $uri.LocalPath -Destination $Destination
        } else {
            Invoke-WebRequest -UseBasicParsing -Uri $uri -OutFile $Destination
        }
    }

    function Move-IntoPlace($Source, $Destination) {
        if ([IO.File]::Exists($Destination)) {
            [IO.File]::Replace($Source, $Destination, [NullString]::Value)
        } else {
            [IO.File]::Move($Source, $Destination)
        }
    }

    function Set-StatusLine($Binary, $Stack) {
        $profileHome = $env:AGY_STATUSLINE_HOME
        if (!$profileHome) { $profileHome = $env:USERPROFILE }
        if (!$profileHome) { $profileHome = $HOME }
        $settingsDir = Join-Path $profileHome '.gemini/antigravity-cli'
        $settingsPath = Join-Path $settingsDir 'settings.json'
        if (Test-Path -LiteralPath $settingsPath) {
            $json = [IO.File]::ReadAllText($settingsPath)
            if (!$json.TrimStart().StartsWith('{')) { throw 'settings.json must contain a JSON object.' }
            $settings = ConvertFrom-Json -InputObject $json
        } else {
            $settings = [pscustomobject]@{}
        }
        $previous = $settings.PSObject.Properties['statusLine']
        if ($null -ne $previous -and $previous.Value -is [pscustomobject]) {
            $status = $previous.Value
        } else {
            $status = [pscustomobject]@{}
        }
        $command = $Binary
        if ($command -match '[\s&()^%!;,=]') { $command = '"' + $command + '"' }
        $status | Add-Member -Force -NotePropertyMembers @{
            type = 'command'
            command = $command
            enabled = $true
        }
        if ($Stack) {
            $status | Add-Member -Force -NotePropertyName stack_with_default -NotePropertyValue $true
        }
        $settings | Add-Member -Force -NotePropertyName statusLine -NotePropertyValue $status
        $json = ConvertTo-Json -InputObject $settings -Depth 100
        [void][IO.Directory]::CreateDirectory($settingsDir)
        $settingsTemp = Join-Path $settingsDir ('.settings.json.' + [Guid]::NewGuid())
        try {
            [IO.File]::WriteAllText($settingsTemp, $json + [Environment]::NewLine)
            Move-IntoPlace $settingsTemp $settingsPath
        } finally {
            [IO.File]::Delete($settingsTemp)
        }
    }

    $repo = 'https://github.com/kmou424/agy-statusline'
    $tagPattern = '\Av[0-9][a-zA-Z0-9._+-]*\z'
    if ($Canary) {
        $tag = 'canary'
    } elseif ($Version) {
        if ($Version -cnotmatch $tagPattern) { throw "Invalid release tag: $Version" }
        $tag = $Version
    } else {
        $tag = $env:AGY_STATUSLINE_LATEST_TAG
        if (!$tag) {
            try {
                $release = Invoke-RestMethod -UseBasicParsing -Uri 'https://api.github.com/repos/kmou424/agy-statusline/releases/latest'
                $tag = $release.tag_name
            } catch {
                throw 'Could not find a stable release. Try -Canary.'
            }
        }
        if ($tag -cnotmatch $tagPattern) { throw 'Could not find a stable release. Try -Canary.' }
    }

    $arch = $env:AGY_STATUSLINE_ARCH
    if (!$arch) { $arch = $env:PROCESSOR_ARCHITEW6432 }
    if (!$arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
    if (!$arch) { $arch = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() }
    switch ($arch) {
        { $_ -in 'AMD64', 'X64' } { $target = 'x86_64-pc-windows-msvc' }
        'ARM64' { $target = 'aarch64-pc-windows-msvc' }
        default { throw "Unsupported architecture: $arch" }
    }
    if (!$InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs/agy-statusline' }
    $InstallDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($InstallDir)
    $base = $env:AGY_STATUSLINE_RELEASE_BASE_URL
    if (!$base) { $base = "$repo/releases/download" }
    $assetDir = $base.TrimEnd('/') + '/' + $tag
    $archive = "agy-statusline-$tag-$target.zip"
    $tmpDir = Join-Path ([IO.Path]::GetTempPath()) ('agy-statusline-' + [Guid]::NewGuid())
    $binaryTemp = $null
    [void][IO.Directory]::CreateDirectory($tmpDir)
    try {
        $zip = Join-Path $tmpDir $archive
        $sums = Join-Path $tmpDir 'SHA256SUMS'
        Get-ReleaseAsset "$assetDir/$archive" $zip
        Get-ReleaseAsset "$assetDir/SHA256SUMS" $sums
        $pattern = '\A([0-9a-fA-F]{64})  ' + [regex]::Escape($archive) + '\z'
        $expected = @(foreach ($line in [IO.File]::ReadAllLines($sums)) {
            if ($line -cmatch $pattern) { $Matches[1] }
        })
        if ($expected.Count -ne 1) { throw "Missing or invalid SHA256SUMS entry for $archive." }
        if ((Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash -ne $expected[0]) {
            throw "SHA-256 mismatch for $archive."
        }

        $extracted = Join-Path $tmpDir 'extracted'
        Expand-Archive -LiteralPath $zip -DestinationPath $extracted
        $binaryName = 'agy-statusline.exe'
        $payload = Join-Path $extracted $binaryName
        if (!(Test-Path -LiteralPath $payload -PathType Leaf)) {
            throw "Archive must contain $binaryName."
        }
        [void][IO.Directory]::CreateDirectory($InstallDir)
        $binary = Join-Path $InstallDir $binaryName
        $binaryTemp = Join-Path $InstallDir ('.agy-statusline.' + [Guid]::NewGuid())
        [IO.File]::Copy($payload, $binaryTemp)
        Move-IntoPlace $binaryTemp $binary
        if (!$NoConfigure) { Set-StatusLine $binary $StackWithDefault }
        Write-Output "Installed $tag to $binary"
    } finally {
        if ($binaryTemp) { [IO.File]::Delete($binaryTemp) }
        Remove-Item -LiteralPath $tmpDir -Recurse -Force
    }
}
