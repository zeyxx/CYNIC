$body = '{"source":"test","reality":"CODE","data":{"snippet":"x=1"},"context":"quick test"}'
$r = Invoke-RestMethod -Method POST -Uri http://localhost:8000/perceive -ContentType 'application/json' -Body $body
$r | ConvertTo-Json -Depth 10
