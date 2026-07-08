package so.cynic.kyon.core.storage

import androidx.room.Entity
import androidx.room.PrimaryKey

/**
 * OrganismStateEntity (Homeostasis)
 * 
 * This entity tracks KYON's internal biological state (Homeostasis) locally.
 * It allows KYON to operate as a sovereign autonomous node. Instead of fetching
 * rules from CYNIC synchronously, KYON evaluates its own pressure and updates its Shield.
 */
@Entity(tableName = "organism_state")
data class OrganismStateEntity(
    @PrimaryKey val id: Int = 1, // Singleton row for the device
    val focusPressure: Float,    // 0.0 to 1.0 (High pressure = strict shield)
    val shieldLevel: String,     // e.g., "PERMISSIVE", "STRICT", "LOCKDOWN"
    val lastVerdict: String?,    // Last verdict from askesis (e.g., "HOWL", "WAG")
    val updatedAtMs: Long,
)
