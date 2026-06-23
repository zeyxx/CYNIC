import httpx
import time
from datetime import datetime, timezone
from src.domain.entities import Target, Observation, ObservationStatus
from src.ports.sensor import SensorPort

class HttpSensorAdapter(SensorPort):
    """
    Adapter that implements SensorPort for HTTP targets (API endpoints, websites).
    """
    def __init__(self, timeout_seconds: float = 10.0):
        self.timeout = timeout_seconds

    async def perceive(self, target: Target) -> Observation:
        start_time = time.perf_counter()
        
        try:
            async with httpx.AsyncClient(timeout=self.timeout) as client:
                response = await client.get(target.uri)
                latency_ms = (time.perf_counter() - start_time) * 1000
                
                # Determine status based on HTTP code
                if response.status_code == 200:
                    status = ObservationStatus.HEALTHY
                elif 500 <= response.status_code < 600:
                    status = ObservationStatus.DOWN
                else:
                    status = ObservationStatus.DEGRADED
                
                return Observation(
                    target_id=target.id,
                    status=status,
                    timestamp=datetime.now(timezone.utc),
                    latency_ms=latency_ms,
                    raw_response={
                        "status_code": response.status_code,
                        "headers": dict(response.headers)
                    }
                )
                
        except httpx.RequestError as e:
            latency_ms = (time.perf_counter() - start_time) * 1000
            return Observation(
                target_id=target.id,
                status=ObservationStatus.DOWN,
                timestamp=datetime.now(timezone.utc),
                latency_ms=latency_ms,
                raw_response={"error": str(e)}
            )
