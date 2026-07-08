package so.cynic.kyon.core.domain

/**
 * L'interface entre le système sensoriel Android (Kotlin) et le Cerveau (Rust).
 * Le "SSoT" (Source of Truth) mathématique est dans le crate kyon-core.
 */
object KyonRustBridge {
    init {
        // Charge la librairie dynamique compilée depuis Rust (libkyon_core.so)
        System.loadLibrary("kyon_core")
    }

    /**
     * Calcule la pression métabolique (Stress/Entropy) d'une session.
     * Logique déportée en Rust pour garantir la sécurité mémoire et les perfs.
     */
    external fun calculateSessionPressure(durationMs: Long): Float

    // --- PIPELINE FORENSIQUE : Ingestion des I/O vers le Tronc Cérébral Rust ---

    external fun ingestScrollEvent(timestampMs: Long, packageName: String)
    external fun ingestSemanticContext(timestampMs: Long, packageName: String, content: String)
    external fun ingestUserTextInput(timestampMs: Long, packageName: String, typedText: String)
    external fun ingestPhysicalSensor(timestampMs: Long, sensorType: String, value: Float)
}
