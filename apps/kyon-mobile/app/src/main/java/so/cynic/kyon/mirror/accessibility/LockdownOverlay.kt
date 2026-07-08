package so.cynic.kyon.mirror.accessibility

import android.content.Context
import android.graphics.PixelFormat
import android.os.Build
import android.view.Gravity
import android.view.LayoutInflater
import android.view.View
import android.view.WindowManager
import android.widget.Button
import android.widget.EditText
import android.widget.TextView

/**
 * KYON Lockdown Overlay
 * Affiche l'aboiement de KYON (généré par Llama.cpp / Candle) par dessus toutes les apps.
 * Bloque l'interaction utilisateur tant qu'une Falsification ou une Acceptation n'a pas été émise.
 */
class LockdownOverlay(private val context: Context) {
    private var windowManager: WindowManager = context.getSystemService(Context.WINDOW_SERVICE) as WindowManager
    private var overlayView: View? = null

    fun showLockdown(kyonBark: String, onResponse: (Boolean, String) -> Unit) {
        if (overlayView != null) return

        // Configuration pour forcer l'Overlay système (Lockdown)
        val layoutParams = WindowManager.LayoutParams(
            WindowManager.LayoutParams.MATCH_PARENT,
            WindowManager.LayoutParams.MATCH_PARENT,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O)
                WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
            else
                WindowManager.LayoutParams.TYPE_PHONE,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
            WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
            WindowManager.LayoutParams.FLAG_DIM_BEHIND, // Assombrit le fond pour le focus
            PixelFormat.TRANSLUCENT
        )
        layoutParams.gravity = Gravity.CENTER
        layoutParams.dimAmount = 0.8f // 80% d'opacité noire en fond

        // Inflate la vue de base (On pourrait injecter une vue Compose ici via ComposeView)
        // Pour être agnostique et direct, nous créons la UI programmatiquement ou via XML
        overlayView = createOverlayView(kyonBark, onResponse)
        
        windowManager.addView(overlayView, layoutParams)
    }

    private fun createOverlayView(kyonBark: String, onResponse: (Boolean, String) -> Unit): View {
        // Idéalement on utilise Jetpack Compose, mais un LinearLayout programmatique fait le job pour l'Architecture
        val view = android.widget.LinearLayout(context).apply {
            orientation = android.widget.LinearLayout.VERTICAL
            setBackgroundColor(android.graphics.Color.parseColor("#121212")) // Dark Mode "Cynic"
            setPadding(64, 128, 64, 64)
            gravity = Gravity.CENTER
        }

        val title = TextView(context).apply {
            text = "🚨 ALERTE CYBERNÉTIQUE"
            setTextColor(android.graphics.Color.RED)
            textSize = 24f
            setTypeface(null, android.graphics.Typeface.BOLD)
        }

        val barkMessage = TextView(context).apply {
            text = kyonBark
            setTextColor(android.graphics.Color.WHITE)
            textSize = 18f
            setPadding(0, 32, 0, 64)
        }

        val falsificationInput = EditText(context).apply {
            hint = "Clarifiez votre intention (Pensées & Paroles alignées avec cette Action)..."
            setHintTextColor(android.graphics.Color.GRAY)
            setTextColor(android.graphics.Color.WHITE)
            setBackgroundColor(android.graphics.Color.parseColor("#222222"))
            minLines = 3
        }

        val acceptBtn = Button(context).apply {
            text = "Accepter (Alignement Restauré)"
            setBackgroundColor(android.graphics.Color.parseColor("#006400")) // Dark Green
            setTextColor(android.graphics.Color.WHITE)
            setOnClickListener {
                onResponse(true, "Accepted")
                dismiss()
            }
        }

        val falsifyBtn = Button(context).apply {
            text = "Falsifier (Soumettre l'Intention à l'Hypergraphe)"
            setBackgroundColor(android.graphics.Color.parseColor("#8B0000")) // Dark Red
            setTextColor(android.graphics.Color.WHITE)
            setOnClickListener {
                onResponse(false, falsificationInput.text.toString())
                dismiss()
            }
        }

        view.addView(title)
        view.addView(barkMessage)
        view.addView(falsificationInput)
        view.addView(acceptBtn)
        view.addView(falsifyBtn)

        return view
    }

    fun dismiss() {
        overlayView?.let {
            windowManager.removeView(it)
            overlayView = null
        }
    }
}
