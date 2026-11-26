# Builds this checkout and starts a title on an NVIDIA card (RTX 30 series or newer). Usually
# called from your own "Start VIctory NVIDIA.cmd" (see NVIDIA.md); it can also be run directly:
#
#   -Game <file>       the title's main executable; a relative path is taken from the repository's
#                      top folder. Else env VICTORY_GAME, else the one picked before on this PC,
#                      else a file picker opens (the choice is remembered per PC).
#   -PickGame          open the file picker even if a title was picked before
#   -RayTracing        ray-traced indirect light (lit shadows)
#   -SeparatePads      each controller its own device
#   -Safe              without the fork's newest CPU shortcuts: steadier, no welcome-screen stepping
#   -SeriesX           Series X mode instead of Series S
#   -LowVram / -FullVram  less video memory for textures and buffer copies, or not. Neither: on for
#                      cards with 8 GB or less, off above.
#   -Release           the release build instead of the diagnostics build
#   -InputTrace        log every controller reading the title asks for
#   -BuildOnly         build and stop
#   -Settings 'A=1 B=2' further VICTORY_* settings, space-separated (they win over the flags)
#
# Saves:  %LOCALAPPDATA%\VIctory-Fork\data-xtx      Cache: %LOCALAPPDATA%\VIctory-Fork\cache-xtx
# Build:  %LOCALAPPDATA%\VIctory-Fork\build-xtx     Logs:  %LOCALAPPDATA%\VIctory-Fork\logs (UTF-16)
param(
    [string]$Game,
    [switch]$PickGame,
    [switch]$RayTracing,
    [switch]$SeparatePads,
    [switch]$Safe,
    [switch]$SeriesX,
    [switch]$LowVram,
    [switch]$FullVram,
    [switch]$Release,
    [switch]$InputTrace,
    [switch]$BuildOnly,
    [string]$Settings = ''
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$home_ = Join-Path $env:LOCALAPPDATA 'VIctory-Fork'
New-Item -ItemType Directory -Force $home_ | Out-Null

# --- The graphics card ----------------------------------------------------------------------
# nvidia-smi comes with the NVIDIA driver. compute_cap: 8.6 = RTX 30, 8.9 = RTX 40, 12.0 = RTX 50.
$gpu = $null
$smi = (Get-Command nvidia-smi -ErrorAction SilentlyContinue).Source
if (-not $smi) { $smi = Join-Path $env:SystemRoot 'System32\nvidia-smi.exe' }
if (Test-Path $smi) {
    $ErrorActionPreference = 'Continue'
    $line = & $smi --query-gpu=name,memory.total,compute_cap,driver_version --format=csv,noheader,nounits 2>$null | Select-Object -First 1
    $ErrorActionPreference = 'Stop'
    if ($line) {
        $f = $line -split ',\s*'
        $gpu = [pscustomobject]@{ Name = $f[0]; MemoryMB = [int]$f[1]; Cap = [double]::Parse($f[2], [Globalization.CultureInfo]::InvariantCulture); Driver = $f[3] }
    }
}
if (-not $gpu) {
    Write-Host 'Warning: no NVIDIA card found (nvidia-smi gave nothing). This launcher is made for RTX 30 series and newer; continuing anyway.' -ForegroundColor Yellow
} else {
    Write-Host ("GPU:   {0}, {1:N1} GB, driver {2}" -f $gpu.Name, ($gpu.MemoryMB / 1024), $gpu.Driver)
    if ($gpu.Cap -lt 8.6) {
        Write-Host "Warning: $($gpu.Name) is older than the RTX 30 series this is made for. It may not run, or only slowly; continuing anyway." -ForegroundColor Yellow
    }
}
if (-not $LowVram -and -not $FullVram -and $gpu -and $gpu.MemoryMB -le 8704) {
    $LowVram = [switch]::Present
    Write-Host 'Low-VRAM settings on (the card has 8 GB or less).'
}

# --- The title ------------------------------------------------------------------------------
if (-not $BuildOnly) {
    $remembered = Join-Path $home_ 'title-path.txt'
    if (-not $Game) { $Game = $env:VICTORY_GAME }
    if (-not $Game -and -not $PickGame -and (Test-Path $remembered)) {
        $Game = (Get-Content $remembered -TotalCount 1).Trim()
        if (-not (Test-Path $Game -PathType Leaf)) { Write-Host "The title picked before is gone ($Game); pick it again."; $Game = $null }
    }
    if (-not $Game) {
        Add-Type -AssemblyName System.Windows.Forms
        $pick = New-Object System.Windows.Forms.OpenFileDialog
        $pick.Title = "VIctory: pick the title's main executable"
        $pick.Filter = 'Programs (*.exe)|*.exe'
        if ($pick.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { throw 'No title picked.' }
        $Game = $pick.FileName
        Set-Content -Path $remembered -Value $Game -Encoding UTF8
        Write-Host "Remembered for this PC: $Game  (use -PickGame to choose another)"
    }
    if (-not [IO.Path]::IsPathRooted($Game)) { $Game = Join-Path $repo $Game }
    if (-not (Test-Path $Game -PathType Leaf)) { throw "'$Game' is not a file. GAME must be the title's main executable." }
    $game = (Resolve-Path $Game).Path
}

# --- Build tools ----------------------------------------------------------------------------
$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) { $cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path $cargo)) {
    throw "Rust is not installed. Install it from https://rustup.rs, or in a terminal: winget install Rustlang.Rustup . Then start again."
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vc = if (Test-Path $vswhere) { & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath }
if (-not $vc) {
    throw ("The Visual Studio C++ build tools are not installed. Install 'Build Tools for Visual Studio' with the " +
        "'Desktop development with C++' workload, or in a terminal: " +
        "winget install Microsoft.VisualStudio.2022.BuildTools --override `"--passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended`" . Then start again.")
}

# --- Build (cargo only rebuilds what changed; a no-op build takes seconds) -------------------
$targetDir = Join-Path $home_ 'build-xtx'
$profile_ = if ($Release) { 'release' } else { 'diag' }
$victory = Join-Path $targetDir "x86_64-pc-windows-msvc\$profile_\victory.exe"
Write-Host "Building VIctory ($profile_) from $repo (the first build takes several minutes)..."
$buildArgs = @('build', '--locked', '-p', 'victory', '--target', 'x86_64-pc-windows-msvc', '--target-dir', $targetDir)
if ($Release) { $buildArgs += '--release' } else { $buildArgs += @('--profile', 'diag', '--features', 'diagnostics') }
Push-Location $repo
try {
    $ErrorActionPreference = 'Continue'
    & $cargo @buildArgs
    $code = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
} finally { Pop-Location }
if ($code -ne 0) {
    if (Get-Process victory -ErrorAction SilentlyContinue) { throw 'The build failed: VIctory is still running. Close it and start again.' }
    throw "The build failed (exit $code). Is the Visual Studio 'Desktop development with C++' workload installed? See NVIDIA.md."
}
if ($BuildOnly) { Write-Host "Built: $victory"; return }

# --- Settings -------------------------------------------------------------------------------
# The fork's own play settings (scripts\play.ps1).
$set = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_D3D12_DRED = 'light'; VICTORY_SKIP_PIPELINES = 'none'
}
if ($Safe) {
    $set.VICTORY_UNDER_WIDE = '0'; $set.VICTORY_LAND_ECHO = '0'
} else {
    $set.VICTORY_THIN = '1'; $set.VICTORY_JOIN_PARTS = '8'; $set.VICTORY_UPLOAD_AHEAD = '1'
}
# The ray-tracing builder's scans read a value every lane of a console wave holds with a first-lane
# read; on 32-lane NVIDIA waves three of its 53 programs do not translate without this, and it runs
# all or none.
# (The structure builder's three programs that need the host wave's own first lane get it by
# default, VICTORY_FIRSTLANE_OWN_ONLY; VICTORY_FIRSTLANE_OWN=1 would give it to every program.)
if ($RayTracing) { $set.VICTORY_BVH_BUILDER = '1'; $set.VICTORY_BVH_QUARANTINE = '0' }
if ($SeparatePads) { $set.VICTORY_ONE_PAD = '0' }
if ($InputTrace) { $set.VICTORY_LOG = 'info,victory_hle::modules::gameinput=trace' }
if ($LowVram) { $set.VICTORY_TEXTURE_ROOM_PERCENT = '70'; $set.VICTORY_CARD_BUFFERS_MB = '256' }
foreach ($pair in ($Settings -split '\s+' | Where-Object { $_ })) {
    $k, $v = $pair -split '=', 2
    if (-not $k.StartsWith('VICTORY_') -or $null -eq $v) { throw "Setting '$pair' is not VICTORY_NAME=value." }
    $set[$k] = $v
}

$data = Join-Path $home_ 'data-xtx'
$cache = Join-Path $home_ 'cache-xtx'
New-Item -ItemType Directory -Force $data, $cache | Out-Null

Get-ChildItem Env:VICTORY_* | ForEach-Object { Remove-Item "Env:$($_.Name)" }
foreach ($k in $set.Keys) { Set-Item "Env:$k" $set[$k] }
$env:VICTORY_GAME_DIR = Split-Path $game
$env:VICTORY_DATA_DIR = $data
$env:VICTORY_CACHE_DIR = $cache

$console = if ($SeriesX) { 'series-x' } else { 'series-s' }
# Each run keeps its own log (the last 10 are kept); %TEMP%\victory-xtx-latest.log is a copy of the newest.
$logs = Join-Path $home_ 'logs'
New-Item -ItemType Directory -Force $logs | Out-Null
Get-ChildItem $logs -Filter 'run-*.log' | Sort-Object LastWriteTime -Descending | Select-Object -Skip 9 | Remove-Item -ErrorAction SilentlyContinue
$what = @(); foreach ($n in 'RayTracing', 'SeparatePads', 'Safe', 'SeriesX', 'LowVram', 'Release') { if ((Get-Variable $n).Value) { $what += $n } }
$log = Join-Path $logs ("run-{0}-{1}.log" -f (Get-Date -Format 'yyyyMMdd-HHmmss'), $(if ($what) { $what -join '-' } else { 'defaults' }))
$crashes = Get-ChildItem $data -Filter 'crashes.log' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
$crashesBefore = if ($crashes) { $crashes.Length } else { 0 }
Write-Host "VIctory (victory-nvidia-fork, NVIDIA): $console; $(if ($what) { $what -join ', ' } else { 'no flags' })"
Write-Host "Title: $game"
Write-Host "Saves: $data"
Write-Host "Log:   $log"
# The log goes to stderr; Windows PowerShell 5.1 wraps each such line as an error record,
# which must not stop the script.
$ErrorActionPreference = 'Continue'
$commit = if (Get-Command git -ErrorAction SilentlyContinue) { git -C $repo rev-parse --short HEAD 2>$null } else { '?' }
"commit $commit; options: $($what -join ' '); settings: $(($set.Keys | Sort-Object | ForEach-Object { "$_=$($set[$_])" }) -join ' ')" |
    Out-File -FilePath $log -Encoding unicode
& $victory run $game --execute --console $console 2>&1 | ForEach-Object { "$_" } | Tee-Object -FilePath $log -Append
$code = $LASTEXITCODE
('VIctory exited with code {0} (0x{0:X8})' -f $code) | Tee-Object -FilePath $log -Append
Copy-Item $log (Join-Path $env:TEMP 'victory-xtx-latest.log') -Force -ErrorAction SilentlyContinue

# A crash is written to data-xtx\<title>\crashes.log; show what this run added.
$crashes = Get-ChildItem $data -Filter 'crashes.log' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
if ($crashes -and $crashes.Length -gt $crashesBefore) {
    $stream = [System.IO.File]::OpenRead($crashes.FullName)
    try {
        $stream.Seek($crashesBefore, 'Begin') | Out-Null
        $added = (New-Object System.IO.StreamReader($stream)).ReadToEnd()
    } finally { $stream.Close() }
    Write-Host ''
    Write-Host 'VIctory reported a crash:' -ForegroundColor Red
    Write-Host $added
    "---- crash report ----`n$added" | Out-File -FilePath $log -Append -Encoding unicode
}
Write-Host ''
Write-Host "Log of this run: $log"
Write-Host 'To share logs, double-click "Collect logs.cmd" (it puts victory-logs.zip on your desktop).'
