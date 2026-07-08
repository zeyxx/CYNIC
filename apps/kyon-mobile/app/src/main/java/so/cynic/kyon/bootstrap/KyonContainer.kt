package so.cynic.kyon.bootstrap

import so.cynic.kyon.adapters.storage.room.RoomActivityEventStore
import so.cynic.kyon.adapters.storage.room.RoomPhoneReportStore
import so.cynic.kyon.adapters.security.Sha256PeerHasher
import so.cynic.kyon.adapters.sync.kernel.KernelObservationSink
import so.cynic.kyon.application.mirror.RecordNotificationEvent
import so.cynic.kyon.application.observability.RecordActivityEvent
import so.cynic.kyon.application.observability.LoadRecentActivityEvents
import so.cynic.kyon.application.observability.ObserveRecentActivityEvents
import so.cynic.kyon.application.observability.ExportActivityEvents
import so.cynic.kyon.application.shield.ShieldDecision
import so.cynic.kyon.application.shield.SavePhoneReport
import so.cynic.kyon.application.shield.ObservePhoneReports
import so.cynic.kyon.application.shield.DeletePhoneReport
import so.cynic.kyon.application.shield.RelabelPhoneReport
import so.cynic.kyon.application.sync.FlushPendingEvents
import so.cynic.kyon.core.domain.HomeostasisEngine
import so.cynic.kyon.core.storage.KyonDatabase
import so.cynic.kyon.core.sync.KernelConfig
import so.cynic.kyon.ports.ActivityEventStore
import so.cynic.kyon.ports.PhoneReportStore
import so.cynic.kyon.ports.ObservationSink

import so.cynic.kyon.adapters.intelligence.HeuristicIntelligenceAdapter
import so.cynic.kyon.ports.LocalIntelligencePort

class KyonContainer(
    database: KyonDatabase,
    kernelConfig: KernelConfig,
) {
    private val peerHasher = Sha256PeerHasher()
    
    val localIntelligencePort: LocalIntelligencePort = HeuristicIntelligenceAdapter()
    
    val homeostasisEngine: HomeostasisEngine = HomeostasisEngine(database.organismState())
    val activityEventStore: ActivityEventStore = RoomActivityEventStore(
        dao = database.activityEvents(),
        homeostasisEngine = homeostasisEngine
    )
    val phoneReportStore: PhoneReportStore = RoomPhoneReportStore(database.phoneReports())
    val observationSink: ObservationSink = KernelObservationSink(kernelConfig)
    val recordActivityEvent: RecordActivityEvent = RecordActivityEvent(activityEventStore)
    val shieldDecision: ShieldDecision = ShieldDecision(
        hasher = peerHasher,
        phoneReportStore = phoneReportStore,
    )
    val observeRecentActivityEvents: ObserveRecentActivityEvents = ObserveRecentActivityEvents(activityEventStore)
    val loadRecentActivityEvents: LoadRecentActivityEvents = LoadRecentActivityEvents(activityEventStore)
    val exportActivityEvents: ExportActivityEvents = ExportActivityEvents(activityEventStore)
    val observePhoneReports: ObservePhoneReports = ObservePhoneReports(phoneReportStore)
    val savePhoneReport: SavePhoneReport = SavePhoneReport(phoneReportStore, peerHasher)
    val deletePhoneReport: DeletePhoneReport = DeletePhoneReport(phoneReportStore)
    val relabelPhoneReport: RelabelPhoneReport = RelabelPhoneReport(phoneReportStore)
    val recordNotificationEvent: RecordNotificationEvent = RecordNotificationEvent(activityEventStore, localIntelligencePort)
    val flushPendingEvents: FlushPendingEvents = FlushPendingEvents(
        eventStore = activityEventStore,
        observationSink = observationSink,
    )
    val observeHomeostasis: so.cynic.kyon.application.organism.ObserveHomeostasis = so.cynic.kyon.application.organism.ObserveHomeostasis(database.organismState())
}
