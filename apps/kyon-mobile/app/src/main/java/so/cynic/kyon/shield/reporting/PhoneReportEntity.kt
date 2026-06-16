package so.cynic.kyon.shield.reporting

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

/**
 * Room entity for persisted user reports on phone numbers.
 *
 * [numberHash] is indexed for fast lookup when the call/SMS screening service checks
 * whether a peer has been previously reported. One report per hash is enforced by the unique index.
 */
@Entity(
    tableName = "phone_reports",
    indices = [Index(value = ["numberHash"], unique = true)],
)
data class PhoneReportEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val numberHash: String,
    val label: String,
    val reportedAtMs: Long,
) {
    fun toModel(): PhoneReport = PhoneReport(
        numberHash = numberHash,
        label = PhoneReportLabel.valueOf(label),
        reportedAtMs = reportedAtMs,
    )
}

fun PhoneReport.toEntity(): PhoneReportEntity = PhoneReportEntity(
    numberHash = numberHash,
    label = label.name,
    reportedAtMs = reportedAtMs,
)
