package so.cynic.kyon.core.sensing

import so.cynic.kyon.core.model.ActivityEvent

interface Sensor {
    suspend fun snapshot(): List<ActivityEvent>
}
