$old = [Environment]::GetEnvironmentVariable('PATH','Machine')
$entries = $old -split ';'
$clean = $entries | Where-Object {
    $_ -ne '' -and
    $_ -notmatch '(?i)python39' -and
    $_ -notmatch '(?i)python310'
}
$newPath = $clean -join ';'
[Environment]::SetEnvironmentVariable('PATH', $newPath, 'Machine')
Write-Output 'Done. Removed Python 3.9 and 3.10 from Machine PATH.'
Write-Output ''
Write-Output 'Remaining Python entries:'
$newPath -split ';' | Select-String -Pattern 'ython' -SimpleMatch
