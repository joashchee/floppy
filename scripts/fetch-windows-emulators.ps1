$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$resources = Join-Path $root "src-tauri/resources"
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("floppy-windows-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null

function Install-Archive($url, $sha256, $archiveName, $sourceName, $destination, $version) {
    $dest = Join-Path $resources $destination
    $marker = Join-Path $dest "VERSION"
    if ((Test-Path $marker) -and (Get-Content -Raw $marker).Trim() -eq $version) {
        Write-Host "$destination $version already present."
        return
    }

    $archive = Join-Path $work $archiveName
    Invoke-WebRequest -Uri $url -OutFile $archive
    $actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
    if ($actual -ne $sha256) {
        throw "$archiveName SHA-256 mismatch: expected $sha256, got $actual"
    }

    $expanded = Join-Path $work ([guid]::NewGuid().ToString())
    Expand-Archive -LiteralPath $archive -DestinationPath $expanded
    $source = Join-Path $expanded $sourceName
    if (!(Test-Path $source -PathType Container)) {
        throw "The $archiveName archive did not contain $sourceName."
    }

    if (Test-Path $dest) {
        Remove-Item -LiteralPath $dest -Recurse -Force
    }
    New-Item -ItemType Directory -Path $dest | Out-Null
    foreach ($item in Get-ChildItem -LiteralPath $source -Force) {
        Copy-Item -LiteralPath $item.FullName -Destination $dest -Recurse -Force
    }
    Set-Content -LiteralPath $marker -Value $version -NoNewline
    Write-Host "Installed $dest"
}

try {
    Install-Archive `
        "https://github.com/dosbox-staging/dosbox-staging/releases/download/v0.83.0/dosbox-staging-windows-x64-v0.83.0.zip" `
        "725b915e325a6d410ce30a10989fd492fdad07f6611fff40ff77f160478810f4" `
        "dosbox-staging.zip" "dosbox-staging-v0.83.0" "dosbox" "v0.83.0 Windows x64"
    Install-Archive `
        "https://github.com/FrodeSolheim/fs-uae/releases/download/v3.2.35/FS-UAE_3.2.35_Windows_x86-64.zip" `
        "b402e2802d518a085c878be96c5c0a656fa3dec57dcf0019d226f3285069063e" `
        "fs-uae.zip" "FS-UAE" "fs-uae" "v3.2.35 Windows x86-64"

    # Basilisk II has no pinned Windows build. Keep the configured resource
    # directory present; users can choose a compatible BasiliskII.exe.
    $basilisk = Join-Path $resources "basilisk"
    New-Item -ItemType Directory -Force -Path $basilisk | Out-Null
    Set-Content -LiteralPath (Join-Path $basilisk "VERSION") -Value "not bundled on Windows" -NoNewline
}
finally {
    Remove-Item -LiteralPath $work -Recurse -Force
}
