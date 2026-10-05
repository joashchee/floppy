$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "Run this script from Windows PowerShell on Windows."
}

$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $root
try {
    & (Join-Path $PSScriptRoot "fetch-windows-emulators.ps1")

    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE ".cargo" }
    $remaps = @(
        "--remap-path-prefix=`"$env:USERPROFILE=~`""
        "--remap-path-prefix=`"$root=.`""
        "--remap-path-prefix=`"$cargoHome=~/.cargo`""
    )
    $env:RUSTFLAGS = (@($env:RUSTFLAGS) + $remaps | Where-Object { $_ }) -join " "

    python (Join-Path $root "scripts/third-party-licenses.py")
    if ($LASTEXITCODE -ne 0) {
        throw "Generating the bundled third-party licenses failed."
    }

    npm run tauri build -- @args
    if ($LASTEXITCODE -ne 0) {
        throw "The Tauri Windows build failed."
    }

    $exe = Join-Path $root "src-tauri/target/release/floppy.exe"
    if (!(Test-Path $exe -PathType Leaf)) {
        throw "The build did not produce $exe."
    }
    foreach ($bundle in @("nsis", "msi")) {
        $directory = Join-Path $root "src-tauri/target/release/bundle/$bundle"
        if (!(Get-ChildItem -LiteralPath $directory -File -ErrorAction SilentlyContinue)) {
            throw "The build did not produce a $bundle installer in $directory."
        }
    }
    $bytes = [System.IO.File]::ReadAllBytes($exe)
    $utf8 = [System.Text.Encoding]::UTF8.GetString($bytes)
    $utf16 = [System.Text.Encoding]::Unicode.GetString($bytes)
    if ($utf8.Contains($env:USERPROFILE) -or $utf16.Contains($env:USERPROFILE)) {
        throw "The release executable still contains the build user's profile path."
    }
    Write-Host "Checked: no $env:USERPROFILE path in $exe"
}
finally {
    Pop-Location
}
