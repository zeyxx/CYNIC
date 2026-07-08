package so.cynic.kyon.mirror.accessibility

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import so.cynic.kyon.KyonApp

/**
 * L'OS dans l'OS : Service cognitif central.
 * KYON utilise ce service pour lire en temps réel les structures sémantiques 
 * affichées à l'écran (textes, boutons, flux) et en extraire le contexte 
 * avant même que l'application hôte n'ait pu manipuler l'utilisateur.
 */
class KyonAccessibilityService : AccessibilityService() {
    private val scope = CoroutineScope(Dispatchers.IO)
    private lateinit var lockdownOverlay: LockdownOverlay
    private val mainHandler = android.os.Handler(android.os.Looper.getMainLooper())

    override fun onServiceConnected() {
        super.onServiceConnected()
        lockdownOverlay = LockdownOverlay(this)
        println("KYON: Service Connecté. L'Organisme est en éveil. Lockdown prêt.")
    }

    // Callback pour déclencher l'overlay depuis Rust via JNI
    @Suppress("unused")
    fun onPressureThresholdReached(kyonBark: String) {
        mainHandler.post {
            lockdownOverlay.showLockdown(kyonBark) { accepted, argument ->
                if (accepted) {
                    println("KYON: Homéostasie restaurée par Acceptation.")
                    // TODO: JNI appel pour ZK-Proof Acceptation
                } else {
                    println("KYON: Tentative de Falsification ZK: $argument")
                    // TODO: JNI appel pour ZK-Proof Falsification
                }
            }
        }
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event == null) return
        
        val app = application as? KyonApp ?: return
        
        // 1. Capture Métadonnées Cinétiques : Vitesse de scroll (Symptôme du Doomscrolling)
        if (event.eventType == AccessibilityEvent.TYPE_VIEW_SCROLLED) {
            val scrollTimestamp = System.currentTimeMillis()
            so.cynic.kyon.core.domain.KyonRustBridge.ingestScrollEvent(scrollTimestamp, packageName)
        }

        // 2. Capture des entrées textuelles de l'utilisateur (Forensique Active)
        if (event.eventType == AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED) {
            val typedText = event.text?.joinToString("") ?: ""
            if (typedText.isNotBlank()) {
                so.cynic.kyon.core.domain.KyonRustBridge.ingestUserTextInput(
                    System.currentTimeMillis(),
                    packageName,
                    typedText
                )
            }
        }

        // On ne traite la sémantique profonde (L'écran entier) que si la feature est active
        if (!app.featureFlags.socialSemanticMonitoringEnabled && !app.featureFlags.commercialSemanticMonitoringEnabled) {
            return
        }

        // 3. Extraction organique des sémantiques (Le pipeline sensoriel)
        val rootNode = rootInActiveWindow ?: return
        
        scope.launch {
            // Parcours récursif de l'arbre UI pour extraire les textes
            val screenTextBuilder = StringBuilder()
            extractTextFromNode(rootNode, screenTextBuilder)
            
            val semanticContext = screenTextBuilder.toString().trim()
            if (semanticContext.isNotEmpty()) {
                so.cynic.kyon.core.domain.KyonRustBridge.ingestSemanticContext(
                    System.currentTimeMillis(),
                    packageName,
                    semanticContext
                )
            }
            
            rootNode.recycle()
        }
    }

    /**
     * Parcours récursif de l'arbre sémantique de l'écran.
     * Cette fonction est le "nerf optique" de l'organisme.
     */
    private fun extractTextFromNode(node: android.view.accessibility.AccessibilityNodeInfo?, builder: StringBuilder) {
        if (node == null) return

        // Capture le texte visible
        if (!node.text.isNullOrBlank()) {
            builder.append(node.text).append(" | ")
        } else if (!node.contentDescription.isNullOrBlank()) {
            // Capture le texte alternatif (souvent utilisé pour les images/boutons)
            builder.append(node.contentDescription).append(" | ")
        }

        for (i in 0 until node.childCount) {
            extractTextFromNode(node.getChild(i), builder)
        }
    }

    override fun onInterrupt() {
        // Le service a été interrompu par le système.
    }
}
