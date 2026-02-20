# Send 25 requests to trigger a flush (F(8)=21 threshold)
$body = '{"source":"test","reality":"CODE","data":{"snippet":"def foo(): pass"},"context":"DB flush test"}'
Write-Host "Sending 25 requests..."
for ($i=1; $i -le 25; $i++) {
  try {
    Invoke-RestMethod -Method POST -Uri http://localhost:8000/perceive -ContentType 'application/json' -Body $body | Out-Null
  } catch {}
}

Start-Sleep 3

# Check introspect
$r = Invoke-RestMethod http://localhost:8000/introspect
Write-Host "pending_flush: $($r.learning.pending_flush)"
Write-Host "total_updates: $($r.learning.total_updates)"
Write-Host "states: $($r.learning.states)"
Write-Host ""

# Check DB directly
Write-Host "=== Checking Postgres q_table ==="
docker exec cynic-postgres psql -U cynic -d cynic_py -c "SELECT count(*) as rows FROM q_table;" 2>&1
