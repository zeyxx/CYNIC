import React from 'react'

const VERDICT_CONFIG = {
  HOWL: { emoji: '🟢', color: 'green', label: 'Excellent (Q ≥ 82)' },
  WAG: { emoji: '🟡', color: 'blue', label: 'Good (Q 61-82)' },
  GROWL: { emoji: '🟠', color: 'amber', label: 'Warning (Q 38-61)' },
  BARK: { emoji: '🔴', color: 'red', label: 'Critical (Q < 38)' }
}

export default function JudgmentDisplay({ judgment }) {
  if (!judgment) {
    return (
      <div className="card">
        <p className="text-gray-400 text-center py-8">Awaiting first judgment...</p>
      </div>
    )
  }

  const config = VERDICT_CONFIG[judgment.verdict] || VERDICT_CONFIG.BARK
  const confidencePercent = (judgment.confidence * 100).toFixed(1)

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
      {/* Main Judgment */}
      <div className="lg:col-span-2">
        <div className="card">
          <h2 className="card-header">⚖️ Current Judgment</h2>

          {/* Verdict Banner */}
          <div className={`bg-${config.color}-900 border border-${config.color}-700 rounded-lg p-6 mb-6`}>
            <div className="flex items-center gap-3 mb-2">
              <span className="text-3xl">{config.emoji}</span>
              <div>
                <div className="text-sm text-gray-400">VERDICT</div>
                <div className="text-2xl font-bold text-gray-50">{judgment.verdict}</div>
              </div>
            </div>
            <p className="text-sm text-gray-300 mt-2">{config.label}</p>
          </div>

          {/* Scores */}
          <div className="grid grid-cols-2 gap-4 mb-6">
            <div className="bg-gray-700 rounded-lg p-4">
              <div className="text-xs text-gray-400 uppercase">Q-Score</div>
              <div className="text-3xl font-bold text-gray-50 mt-1">{judgment.q_score?.toFixed(1) || '—'}</div>
              <div className="text-xs text-gray-400 mt-1">max 100.0</div>
            </div>

            <div className="bg-gray-700 rounded-lg p-4">
              <div className="text-xs text-gray-400 uppercase">Confidence</div>
              <div className="text-3xl font-bold text-gray-50 mt-1">{confidencePercent}%</div>
              <div className="text-xs text-gray-400 mt-1">max 61.8% (φ⁻¹)</div>
            </div>
          </div>

          {/* Reasoning */}
          {judgment.reasoning && (
            <div>
              <h3 className="text-sm font-semibold text-gray-300 mb-2">Reasoning</h3>
              <p className="text-sm text-gray-400 leading-relaxed bg-gray-700 rounded-lg p-3">
                {judgment.reasoning}
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Metadata & Details */}
      <div className="space-y-4">
        <div className="card">
          <h3 className="card-header text-sm">Judgment Info</h3>
          <div className="space-y-2 text-xs">
            <div>
              <div className="text-gray-400">ID</div>
              <div className="text-gray-300 font-mono truncate">{judgment.id?.substring(0, 16)}...</div>
            </div>
            <div>
              <div className="text-gray-400">Cell ID</div>
              <div className="text-gray-300 font-mono truncate">{judgment.cell_id?.substring(0, 16)}...</div>
            </div>
            <div>
              <div className="text-gray-400">Source</div>
              <div className="text-gray-300">{judgment.source || 'unknown'}</div>
            </div>
          </div>
        </div>

        <div className="card">
          <h3 className="card-header text-sm">Φ-Metrics</h3>
          <div className="space-y-2 text-xs">
            <div className="flex justify-between">
              <span className="text-gray-400">φ-Bound Compliance</span>
              <span className="text-green-400">✓</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Confidence Φ⁻¹</span>
              <span className="text-gray-300">{(0.618 * 100).toFixed(1)}% max</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Current</span>
              <span className="text-gray-300">{confidencePercent}%</span>
            </div>
          </div>
        </div>

        <div className="card bg-purple-900 border-purple-700">
          <p className="text-xs text-purple-200">
            <span className="font-semibold">Understanding</span><br/>
            Every judgment includes Dogs' individual votes, φ-bounded confidence, and reasoning trace
          </p>
        </div>
      </div>
    </div>
  )
}
