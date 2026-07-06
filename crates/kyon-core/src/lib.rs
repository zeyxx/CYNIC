pub mod llm;
pub mod orchestrator;

use jni::objects::JClass;
use jni::sys::{jfloat, jlong};
use jni::JNIEnv;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

/// DATA IS QUEEN - Le Buffer Sensoriel Temporel
/// Ce buffer accumule les métadonnées (scrolls, textes) sur une fenêtre glissante.
/// Il permet de ne pas surcharger le modèle LLM et de compresser le contexte.
pub struct SensoryWindow {
    pub scroll_count: u32,
    pub recent_semantics: VecDeque<String>,
    pub recent_inputs: VecDeque<String>,
    pub window_start_ms: jlong,
}

impl SensoryWindow {
    fn new(timestamp: jlong) -> Self {
        Self {
            scroll_count: 0,
            recent_semantics: VecDeque::with_capacity(10), // Garde les 10 dernières captures
            recent_inputs: VecDeque::with_capacity(10),
            window_start_ms: timestamp,
        }
    }
}

// L'état global du cerveau (Thread-safe)
static SENSORY_BUFFER: OnceLock<Mutex<SensoryWindow>> = OnceLock::new();

fn get_buffer() -> &'static Mutex<SensoryWindow> {
    SENSORY_BUFFER.get_or_init(|| Mutex::new(SensoryWindow::new(0)))
}

/// Le Cerveau de l'Organisme KYON.
/// Calcule la pression métabolique (Stress/Entropy) d'une session en Rust pur.
/// Cette fonction remplace l'ancienne heuristique Kotlin.
#[no_mangle]
pub extern "system" fn Java_so_cynic_kyon_core_domain_KyonRustBridge_calculateSessionPressure(
    mut _env: JNIEnv,
    _class: JClass,
    duration_ms: jlong,
    _interaction_count: jlong,
    is_social: bool,
    is_commercial: bool,
    is_productive: bool,
) -> jfloat {
    // Constante Universelle CYNIC : L'asymptote épistémique.
    // Au-delà de ce seuil (φ⁻¹ = 0.618), l'organisme considère qu'il y a une rupture de souveraineté.
    const PHI_INVERSE: f32 = 0.618;

    let mut pressure: f32 = 0.1; // Baseline
    let duration_minutes = duration_ms as f32 / 60000.0;

    // Le temps passé sur des apps algorithmiques augmente drastiquement la pression
    if is_social || is_commercial {
        pressure += (duration_minutes * 0.05).min(0.4);
    }

    // La production (Askesis) réduit la pression
    if is_productive {
        pressure -= (duration_minutes * 0.02).min(0.2);
    }

    // Le Ring Buffer : Si la vitesse de scroll est élevée, on augmente la pression
    if let Ok(buffer) = get_buffer().lock() {
        if buffer.window_start_ms > 0 {
            let elapsed_sec = (duration_ms - buffer.window_start_ms) as f32 / 1000.0;
            if elapsed_sec > 0.0 {
                let scroll_freq = buffer.scroll_count as f32 / elapsed_sec;
                // Si plus de 1 scroll par seconde (doomscrolling frénétique)
                if scroll_freq > 1.0 {
                    pressure += 0.2;
                }
            }
        }
    }

    // La pression ne peut pas dépasser 1.0, mais la zone critique (LOCKDOWN) est à 0.618
    pressure.clamp(0.0, 1.0)
}

// --- PIPELINE FORENSIQUE : Ingestion des flux sensoriels ---

#[no_mangle]
pub extern "system" fn Java_so_cynic_kyon_core_domain_KyonRustBridge_ingestScrollEvent(
    mut _env: JNIEnv,
    _class: JClass,
    timestamp_ms: jlong,
    _package_name: jni::objects::JString,
) {
    if let Ok(mut buffer) = get_buffer().lock() {
        buffer.scroll_count += 1;
        if buffer.window_start_ms == 0 {
            buffer.window_start_ms = timestamp_ms;
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_so_cynic_kyon_core_domain_KyonRustBridge_ingestSemanticContext(
    mut env: JNIEnv,
    _class: JClass,
    timestamp_ms: jlong,
    _package_name: jni::objects::JString,
    content: jni::objects::JString,
) {
    if let Ok(mut buffer) = get_buffer().lock() {
        if let Ok(text) = env.get_string(&content) {
            let str_val = text.to_string_lossy().to_string();
            if buffer.recent_semantics.len() >= 10 {
                buffer.recent_semantics.pop_front();
            }
            buffer.recent_semantics.push_back(str_val);
            if buffer.window_start_ms == 0 {
                buffer.window_start_ms = timestamp_ms;
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_so_cynic_kyon_core_domain_KyonRustBridge_ingestUserTextInput(
    mut env: JNIEnv,
    _class: JClass,
    timestamp_ms: jlong,
    _package_name: jni::objects::JString,
    typed_text: jni::objects::JString,
) {
    if let Ok(mut buffer) = get_buffer().lock() {
        if let Ok(text) = env.get_string(&typed_text) {
            let str_val = text.to_string_lossy().to_string();
            if buffer.recent_inputs.len() >= 10 {
                buffer.recent_inputs.pop_front();
            }
            buffer.recent_inputs.push_back(str_val);
            if buffer.window_start_ms == 0 {
                buffer.window_start_ms = timestamp_ms;
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_so_cynic_kyon_core_domain_KyonRustBridge_ingestPhysicalSensor(
    mut _env: JNIEnv,
    _class: JClass,
    _timestamp_ms: jlong,
    _sensor_type: jni::objects::JString,
    _value: jfloat,
) {
    // TODO: Fusion spatio-temporelle (GPS, Luminosité, Accéléromètre, Micro-Tension)
}
