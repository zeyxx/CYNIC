import asyncio
from src.domain.entities import Target, TargetType
from src.adapters.http_sensor import HttpSensorAdapter

async def main():
    # 1. Initialize the adapter (Driven Port)
    sensor = HttpSensorAdapter(timeout_seconds=5.0)

    # 2. Define the domain entity (Target)
    holdex_target = Target(
        id="partner_holdex_live",
        target_type=TargetType.API_ENDPOINT,
        uri="https://holdex.live",
        metadata={"description": "Holdex UI Frontend"}
    )

    print(f"[*] Radar starting scan on: {holdex_target.uri}")

    # 3. Execute the observation
    observation = await sensor.perceive(holdex_target)

    # 4. Output the result in standard JSON format (Constitution Compliant)
    import json
    import dataclasses
    
    # Custom serializer to handle datetime and enums
    def json_serializer(obj):
        if hasattr(obj, 'isoformat'):
            return obj.isoformat()
        if hasattr(obj, 'value'):
            return obj.value
        raise TypeError(f"Type {type(obj)} not serializable")

    print("\n[*] Observation Results:")
    print(json.dumps(dataclasses.asdict(observation), default=json_serializer, indent=2))

if __name__ == "__main__":
    asyncio.run(main())
