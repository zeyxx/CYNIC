package so.cynic.kyon.ports

import so.cynic.kyon.core.model.ActivityEvent

interface ObservationSink {
    suspend fun post(event: ActivityEvent): Boolean
}
