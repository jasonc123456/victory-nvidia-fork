# Puts what is needed to diagnose a run into victory-logs.zip on the desktop:
# the last 5 run logs, VIctory's crash log, the GPU and driver, and Windows' crash records for victory.exe.
# Nothing is sent anywhere; you choose whether to share the zip.
$ErrorActionPreference = 'Continue'
$home_ = Join-Path $env:LOCALAPPDATA 'VIctory-Fork'
$out = Join-Path ([Environment]::GetFolderPath('Desktop')) 'victory-logs.zip'
$work = Join-Path $env:TEMP ('victory-logs-' + [guid]::NewGuid())
New-Item -ItemType Directory -Force $work | Out-Null

Get-ChildItem (Join-Path $home_ 'logs') -Filter 'run-*.log' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 5 | Copy-Item -Destination $work
Get-ChildItem (Join-Path $home_ 'data-xtx') -Filter 'crashes.log' -Recurse -ErrorAction SilentlyContinue |
    Select-Object -First 1 | Copy-Item -Destination $work

$info = Join-Path $work 'system.txt'
& {
    'GPU and driver:'
    Get-CimInstance Win32_VideoController | Format-List Name, DriverVersion, DriverDate, AdapterRAM, CurrentHorizontalResolution, CurrentVerticalResolution | Out-String
    'nvidia-smi:'
    $smi = (Get-Command nvidia-smi -ErrorAction SilentlyContinue).Source
    if ($smi) { & $smi 2>&1 | Out-String } else { 'not found' }
    'CPU and memory:'
    Get-CimInstance Win32_Processor | Format-List Name, NumberOfCores, NumberOfLogicalProcessors | Out-String
    '{0:N1} GB RAM' -f ((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)
    'Windows:'
    (Get-CimInstance Win32_OperatingSystem).Caption + ' ' + (Get-CimInstance Win32_OperatingSystem).Version
    'Drives:'
    Get-PhysicalDisk -ErrorAction SilentlyContinue | Format-Table FriendlyName, MediaType, BusType, Size | Out-String
    'Windows crash records for victory.exe (last 10):'
    Get-WinEvent -FilterHashtable @{ LogName = 'Application'; Id = 1000, 1001, 1002 } -MaxEvents 200 -ErrorAction SilentlyContinue |
        Where-Object { $_.Message -like '*victory*' } | Select-Object -First 10 |
        Format-List TimeCreated, Id, Message | Out-String -Width 300
} | Out-File -FilePath $info -Encoding utf8

Remove-Item $out -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $work '*') -DestinationPath $out
Remove-Item $work -Recurse -Force
Write-Host "Saved: $out"
