"""
CYNIC Nervous System Layer (Layer 3) — Event distribution and neural communication.

Part of the 10-layer organism architecture, the nervous system is the
internal communication backbone that coordinates all organism components.

Event Buses:
    Judgment Bus: Carries judgment decisions through the pipeline
    Sensory Bus: Distributes environmental input events
    Actuator Bus: Routes actions to embodiment endpoints

Communication Patterns:
    Pub/Sub: Asynchronous broadcast (most events)
    Request/Reply: Synchronous RPC for critical paths
    Streaming: Event sequences for learning and audit

Integration Points:
    Judges publish Dog verdict events
    Learner subscribes to outcome feedback
    Actuators subscribe to judgment decisions
    Sensors publish environmental changes

Typical usage:
    from cynic.kernel.organism.layers import NervousSystem
    nervous = NervousSystem()
    await nervous.emit('judgment', judgment)

See Also:
    cynic.kernel.organism.layers: 10-layer organism architecture
    cynic.kernel.core.event_bus: EventBus implementation
    cynic.nervous: Alternative nervous system components
"""
