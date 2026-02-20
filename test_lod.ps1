$body = '{"source":"test","reality":"CODE","data":{"snippet":"x=1"},"context":"quick test"}'
for ($i=1; $i -le 8; $i++) {
  try {
    $r = Invoke-RestMethod -Method POST -Uri http://localhost:8000/perceive -ContentType 'application/json' -Body $body
    Write-Host "REFLEX #$i level=$($r.level_used) Q=$($r.q_score) dur=$($r.duration_ms)ms"
  } catch {
    Write-Host "Error #$i $($_.Exception.Message)"
  }
  Start-Sleep 1
}

Write-Host ""
Write-Host "=== LOD STATE ==="
$lod = Invoke-RestMethod -Uri http://localhost:8000/introspect
$l = $lod.lod
Write-Host "current_lod: $($l.current_lod) ($($l.current_name))"
Write-Host "healthy_streak: $($l.healthy_streak) / $($l.hysteresis_n)"
Write-Host "transitions: $($l.total_transitions)"
$l.recent_transitions | ForEach-Object {
  Write-Host "  $($_.from) -> $($_.to) lat=$($_.latency_ms)ms"
}
