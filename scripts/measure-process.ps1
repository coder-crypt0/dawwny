param(
    [Parameter(Mandatory = $true)][int]$ProcessId,
    [ValidateRange(1, 30)][int]$Seconds = 5,
    [ValidateSet('idle', 'playback')][string]$Phase = 'idle'
)
$ErrorActionPreference = 'Stop'
$initial = Get-Process -Id $ProcessId
$cpuStart = $initial.CPU
$watch = [Diagnostics.Stopwatch]::StartNew()
$samples = @()
while ($watch.Elapsed.TotalSeconds -lt $Seconds) {
    $current = Get-Process -Id $ProcessId
    $samples += [pscustomobject]@{ working = $current.WorkingSet64; private = $current.PrivateMemorySize64 }
    Start-Sleep -Milliseconds 250
}
$final = Get-Process -Id $ProcessId
[pscustomobject]@{
    phase = $Phase
    seconds = [math]::Round($watch.Elapsed.TotalSeconds, 3)
    working_set_mib = [math]::Round(($samples.working | Measure-Object -Average).Average / 1MB, 2)
    private_mib = [math]::Round(($samples.private | Measure-Object -Average).Average / 1MB, 2)
    peak_private_mib = [math]::Round(($samples.private | Measure-Object -Maximum).Maximum / 1MB, 2)
    cpu_percent_of_one_core = [math]::Round(100 * ($final.CPU - $cpuStart) / $watch.Elapsed.TotalSeconds, 2)
} | ConvertTo-Json
