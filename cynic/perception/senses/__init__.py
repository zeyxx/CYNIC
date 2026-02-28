"""
CYNIC Senses — Environmental perception and input processing.

Provides sensory interfaces for CYNIC to perceive external events:
- Discord messages and reactions
- Telegram updates
- HTTP webhooks
- Direct API calls

Workers process sensory input asynchronously, converting raw events
into unified Cell objects that feed into the judgment pipeline.

Components:
    workers: Sensory worker implementations (Discord, Telegram, HTTP)
    input_adapter: Converts raw events to Cell objects
    sensory_router: Routes input by type to appropriate workers

Typical usage:
    from cynic.perception.senses import DiscordSensorWorker
    worker = DiscordSensorWorker(token='...')
    cell = await worker.process_message(message)

See Also:
    cynic.discord: Discord bot implementation
    cynic.telegram: Telegram bot implementation
    cynic.brain.cognition: Processes sensory input (Cell) through judgment
"""
