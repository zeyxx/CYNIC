/**
 * CYNIC Training Pipeline Configuration
 *
 * Dual-path: LOCAL (CPU + llama.cpp) and CLOUD (GPU + Unsloth).
 * φ-aligned: learning rates, KL penalties, reward bounds all reference golden ratio.
 *
 * LOCAL path (default):
 *   Qwen 2.5 1.5B → llama.cpp LoRA on CPU → Q4_K_M → Ollama
 *   Cost: $0 | Hardware: any CPU with 16GB RAM
 *
 * CLOUD path (optional):
 *   Qwen 2.5 7B → Unsloth QLoRA on GPU → GGUF → Ollama
 *   Cost: ~$3-5 | Hardware: RTX 4090 / A100 (RunPod/Vast.ai)
 *
 * Pipeline:
 *   1. Export JSONL (export-training-data.mjs)
 *   2. Split data (split-data.mjs)
 *   3. SFT warm-up (teaches CYNIC format)
 *   4. GRPO fine-tune (teaches CYNIC judgment via rewards)
 *   5. Eval (validate before deploying as Dog 0)
 *   6. Deploy → Ollama (CollectiveLearner picks up new model)
 *
 * "φ distrusts φ" — even training is bounded
 *
 * @module cynic/training/config
 */

// Golden ratio constants
const PHI = 1.618033988749895;
const PHI_INV = 0.6180339887498949;   // φ⁻¹ = 1/φ  ≈ 0.618
const PHI_INV_2 = 0.3819660112501051; // φ⁻² = 1/φ² ≈ 0.382
const PHI_INV_3 = 0.2360679774997897; // φ⁻³ = 1/φ³ ≈ 0.236

// ═══════════════════════════════════════════════════════════════════════════
// PROFILES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Training profiles: LOCAL (CPU) vs CLOUD (GPU)
 *
 * Select via CLI: --profile local|cloud
 * Default: local (no GPU required)
 */
export const profiles = {
  local: {
    name: 'local',
    description: 'CPU training via llama.cpp LoRA — free, slower, 1.5B model',
    model: 'Qwen/Qwen2.5-1.5B-Instruct',
    gguf: 'Qwen2.5-1.5B-Instruct-Q4_K_M.gguf',
    engine: 'llama.cpp',
    requiresGpu: false,
    estimatedTime: '4-8h SFT + 12-24h GRPO',
    estimatedCost: '$0',
  },
  cloud: {
    name: 'cloud',
    description: 'GPU training via Unsloth QLoRA — fast, ~$3-5, 7B model',
    model: 'Qwen/Qwen2.5-7B-Instruct',
    gguf: null,  // Unsloth outputs safetensors, convert after
    engine: 'unsloth',
    requiresGpu: true,
    estimatedTime: '1-3h total',
    estimatedCost: '$3-5 (RunPod/Vast.ai)',
  },
};

/** Active profile (overridden by CLI --profile flag) */
export const activeProfile = process.env.CYNIC_TRAIN_PROFILE || 'local';

// ═══════════════════════════════════════════════════════════════════════════
// MODEL (derived from active profile)
// ═══════════════════════════════════════════════════════════════════════════

export const model = {
  /** Base model for fine-tuning (profile-dependent) */
  base: profiles[activeProfile]?.model || profiles.local.model,

  /** Output model name (registered in Ollama after training) */
  outputName: 'cynic-dog0',

  /** Quantization for Ollama deployment */
  quantization: 'Q4_K_M',

  /** Max sequence length */
  maxSeqLength: activeProfile === 'cloud' ? 512 : 384,  // smaller for 1.5B

  /** Chat template (Qwen2.5 uses ChatML) */
  chatTemplate: 'chatml',
};

// ═══════════════════════════════════════════════════════════════════════════
// DATA SPLITS
// ═══════════════════════════════════════════════════════════════════════════

export const data = {
  /** Source JSONL (output of export-training-data.mjs) */
  sourcePattern: 'training-data-*.jsonl',

  /** Output directory for split files */
  outputDir: 'training-splits',

  /** Train/validation/test split ratios (φ-aligned) */
  splits: {
    train: PHI_INV,             // 61.8%
    validation: PHI_INV_2 / 2,  // 19.1%
    test: PHI_INV_3 - 0.045,    // 19.1% (balance to ~100%)
  },

  /** Minimum records required to start training */
  minRecords: 89,  // Fib(11) — below this, SFT won't generalize

  /** Minimum feedback-bearing records for GRPO */
  minFeedbackRecords: 34,  // Fib(9) — need reward signal

  /** Stratification: ensure each verdict type is represented */
  stratifyBy: 'verdict',

  /** Filter: SFT uses all records for format learning; GRPO generates own rewards */
  excludeNeutralRewards: false,

  /** Max records per batch file (for memory management) */
  maxPerFile: 10000,
};

// ═══════════════════════════════════════════════════════════════════════════
// STAGE 1: SFT — LOCAL (llama.cpp CPU LoRA)
// ═══════════════════════════════════════════════════════════════════════════

export const sftLocal = {
  /** Training engine */
  engine: 'llama.cpp',

  /** Path to llama.cpp finetune binary */
  binary: process.env.LLAMA_CPP_PATH || 'llama-finetune',

  /** CPU threads (Ryzen 7 5700G = 16 threads) */
  threads: parseInt(process.env.CYNIC_TRAIN_THREADS, 10) || 14,

  /** Learning rate (Adam alpha in llama.cpp) — φ⁻¹ × 1e-3 */
  adamAlpha: 3e-4,  // llama.cpp default sweet spot

  /** LoRA rank */
  loraR: 8,  // Lower rank for 1.5B (less params to learn)

  /** LoRA alpha (scaling) */
  loraAlpha: 16,

  /** Batch size (limited by RAM) */
  batchSize: 4,

  /** Context length for training */
  contextLength: 384,

  /** Number of training epochs */
  epochs: 3,

  /** Save checkpoint every N iterations */
  saveEvery: 100,

  /** Use memory mapping (required for CPU efficiency) */
  mmap: true,

  /** Sample start token for ChatML */
  sampleStart: '<|im_start|>',

  /** Checkpoint directory */
  checkpointDir: 'training-checkpoints/sft-local',
};

// ═══════════════════════════════════════════════════════════════════════════
// STAGE 1: SFT — CLOUD (Unsloth QLoRA GPU)
// ═══════════════════════════════════════════════════════════════════════════

export const sftCloud = {
  /** Training engine */
  engine: 'unsloth',

  /** Whether to run SFT before GRPO (recommended for cold start) */
  enabled: true,

  /** Learning rate: φ⁻¹ × 1e-4 ≈ 6.18e-5 */
  learningRate: PHI_INV * 1e-4,

  /** Warmup ratio (fraction of steps) */
  warmupRatio: PHI_INV_3,  // 23.6%

  /** Batch sizes */
  perDeviceBatchSize: 4,
  gradientAccumulationSteps: 8,
  effectiveBatchSize: 32,  // 4 × 8

  /** Epochs: 3 for warm-up (don't overfit on format) */
  epochs: 3,

  /** Weight decay: φ⁻³ */
  weightDecay: PHI_INV_3,

  /** Max gradient norm */
  maxGradNorm: 1.0,

  /** LR scheduler */
  scheduler: 'cosine',

  /** LoRA configuration (PEFT — parameter-efficient) */
  lora: {
    enabled: true,
    r: 16,           // Rank
    alpha: 32,       // Scaling factor
    dropout: 0.05,
    targetModules: ['q_proj', 'k_proj', 'v_proj', 'o_proj', 'gate_proj', 'up_proj', 'down_proj'],
    taskType: 'CAUSAL_LM',
  },

  /** 4-bit quantization (QLoRA) */
  quantization: {
    loadIn4bit: true,
    bnbType: 'nf4',
    computeDtype: 'bfloat16',
  },

  /** Save checkpoints every N steps */
  saveSteps: 100,

  /** Evaluation during training */
  evalSteps: 50,

  /** Mixed precision */
  fp16: false,
  bf16: true,

  /** Checkpoint directory */
  checkpointDir: 'training-checkpoints/sft-cloud',
};

/** Profile-aware SFT accessor */
export const sft = activeProfile === 'cloud' ? sftCloud : sftLocal;

// ═══════════════════════════════════════════════════════════════════════════
// STAGE 2: GRPO (Group Relative Policy Optimization)
// ═══════════════════════════════════════════════════════════════════════════

export const grpo = {
  /** Whether to run GRPO after SFT */
  enabled: true,

  /** Learning rate: lower than SFT (φ⁻² × 1e-4 ≈ 3.82e-5) */
  learningRate: PHI_INV_2 * 1e-4,

  /** Group size: number of completions per prompt to compare */
  groupSize: activeProfile === 'cloud' ? 8 : 4,  // Fib(6) or Fib(3) for CPU

  /** KL penalty coefficient: φ⁻² (penalize deviation from SFT model) */
  klCoeff: PHI_INV_2,

  /** Reward normalization */
  normalizeRewards: true,

  /** Reward clipping: [-φ, +φ⁻¹] (match reward function bounds) */
  rewardClipMin: -PHI,
  rewardClipMax: PHI_INV,

  /** Batch sizes */
  perDeviceBatchSize: activeProfile === 'cloud' ? 2 : 1,
  gradientAccumulationSteps: 8,
  effectiveBatchSize: activeProfile === 'cloud' ? 16 : 8,

  /** Max training steps (not epochs — RL is step-based) */
  maxSteps: activeProfile === 'cloud' ? 500 : 200,

  /** Temperature for generation during training */
  temperature: 0.7,

  /** Top-p sampling */
  topP: 0.9,

  /** Max new tokens per generation (keep judgments short) */
  maxNewTokens: 256,

  /** Weight decay */
  weightDecay: PHI_INV_3,

  /** LR scheduler */
  scheduler: 'cosine',

  /** Mixed precision (cloud only) */
  bf16: activeProfile === 'cloud',

  /** Save checkpoints */
  saveSteps: 100,
  evalSteps: 50,

  /**
   * GRPO on CPU (local path):
   * llama.cpp doesn't natively support GRPO, so for local path
   * we use iterative SFT with reward-weighted sampling:
   *   1. Generate N completions per prompt
   *   2. Score each with reward function
   *   3. Keep top completions (reward > 0)
   *   4. Re-train SFT on filtered set
   *   5. Repeat for maxSteps iterations
   * This is "poor man's GRPO" — rejection sampling + SFT.
   */
  localStrategy: 'rejection_sampling_sft',
};

// ═══════════════════════════════════════════════════════════════════════════
// EVALUATION
// ═══════════════════════════════════════════════════════════════════════════

export const evaluation = {
  /** Metrics to compute on test set */
  metrics: {
    /** Score accuracy: |predicted_q_score - actual_q_score| < threshold */
    scoreAccuracy: {
      thresholds: [5, 10, 20],  // Within 5, 10, 20 points
      targetAccuracy: activeProfile === 'cloud' ? PHI_INV_2 : PHI_INV_3,
    },

    /** Verdict agreement: predicted verdict matches actual */
    verdictAgreement: {
      target: activeProfile === 'cloud' ? PHI_INV : PHI_INV_2,
    },

    /** Confidence calibration: predicted confidence correlates with accuracy */
    confidenceCalibration: {
      /** Expected Calibration Error target */
      eceTarget: PHI_INV_3,  // ECE < 23.6%
      bins: 10,
    },

    /** Reward distribution: mean reward should be positive */
    rewardDistribution: {
      targetMeanReward: 0.1,  // Positive signal
      maxNegativeRatio: PHI_INV,  // At most 61.8% negative (early training)
    },

    /** Format compliance: output parseable as JSON judgment */
    formatCompliance: {
      target: 0.9,  // 90% of outputs must be valid JSON
    },
  },

  /** Minimum eval metrics to consider model deployable */
  deploymentGate: {
    /** Must achieve ALL of these to deploy */
    verdictAgreement: PHI_INV_3,      // At least 23.6% (relaxed for 1.5B)
    formatCompliance: 0.7,             // At least 70% parseable
    meanReward: 0.0,                   // Non-negative mean reward
    eceBelow: PHI_INV,                // Calibration error < 61.8%
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// DEPLOYMENT
// ═══════════════════════════════════════════════════════════════════════════

export const deployment = {
  /** Ollama model registration */
  ollama: {
    endpoint: process.env.OLLAMA_URL || 'http://localhost:11434',
    modelName: 'cynic-dog0',

    /** Modelfile template for Ollama */
    modelfileTemplate: `FROM {gguf_path}
TEMPLATE """{{- if .System }}<|im_start|>system
{{ .System }}<|im_end|>
{{ end }}<|im_start|>user
{{ .Prompt }}<|im_end|>
<|im_start|>assistant
"""
PARAMETER temperature 0.3
PARAMETER num_predict 256
PARAMETER stop "<|im_end|>"
SYSTEM "You are CYNIC (κυνικός), a cynical judgment system. Score items on 25 dimensions across 4 axioms (PHI, VERIFY, CULTURE, BURN). Your confidence never exceeds 61.8% (φ⁻¹). Be direct, skeptical, honest."`,
  },

  /** Environment variable to update after deployment */
  envVar: 'CYNIC_DOG0_MODEL',

  /** Rollback: if Dog 0 accuracy drops below this, revert to heuristic */
  rollbackThreshold: PHI_INV_3,  // 23.6% accuracy → rollback
};

// ═══════════════════════════════════════════════════════════════════════════
// FULL PIPELINE (profile-aware)
// ═══════════════════════════════════════════════════════════════════════════

export const pipeline = {
  /** Active profile */
  profile: activeProfile,

  stages: [
    {
      name: 'export',
      script: 'scripts/training/export-training-data.mjs',
      args: ['--min-feedback', 'false'],
      description: 'Export JSONL from PostgreSQL',
      requiresGpu: false,
    },
    {
      name: 'split',
      script: 'scripts/training/split-data.mjs',
      args: [],
      description: 'Split into train/val/test per config ratios',
      requiresGpu: false,
    },
    {
      name: 'sft',
      script: activeProfile === 'cloud'
        ? 'scripts/training/run-sft.sh'
        : 'scripts/training/run-sft-local.sh',
      args: [],
      description: activeProfile === 'cloud'
        ? 'SFT warm-up with Unsloth QLoRA (GPU)'
        : 'SFT warm-up with llama.cpp LoRA (CPU)',
      requiresGpu: activeProfile === 'cloud',
    },
    {
      name: 'grpo',
      script: activeProfile === 'cloud'
        ? 'scripts/training/run-grpo.sh'
        : 'scripts/training/run-grpo-local.mjs',
      args: [],
      description: activeProfile === 'cloud'
        ? 'GRPO reinforcement with reward function (GPU)'
        : 'Rejection sampling + SFT with rewards (CPU)',
      requiresGpu: activeProfile === 'cloud',
    },
    {
      name: 'eval',
      script: 'scripts/training/evaluate.mjs',
      args: [],
      description: 'Evaluate on test set against deployment gate',
      requiresGpu: false,
    },
    {
      name: 'deploy',
      script: 'scripts/training/deploy-ollama.sh',
      args: [],
      description: 'Register quantized model in Ollama',
      requiresGpu: false,
    },
  ],
};

// ═══════════════════════════════════════════════════════════════════════════
// EXPORT ALL
// ═══════════════════════════════════════════════════════════════════════════

export default {
  profiles,
  activeProfile,
  model,
  data,
  sft,
  sftLocal,
  sftCloud,
  grpo,
  evaluation,
  deployment,
  pipeline,
};
