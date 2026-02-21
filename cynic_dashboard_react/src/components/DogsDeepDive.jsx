import React from 'react'

const DOG_DESCRIPTIONS = {
  ANALYST: 'Pattern detector. Finds code patterns, anomalies, metrics.',
  ARCHITECT: 'Design reviewer. Evaluates architecture, modularity, SOLID.',
  CARTOGRAPHER: 'Dependency mapper. Traces imports, coupling, DAG health.',
  CYNIC: 'Truth keeper. Meta-judger. Questions everything including self.',
  DEPLOYER: 'Readiness assessor. Tests, coverage, CI/CD, deployment risk.',
  GUARDIAN: 'Safety enforcer. Security, permissions, injection, escapes.',
  JANITOR: 'Code cleaner. Orphans, dead code, style, naming conventions.',
  ORACLE: 'Future predictor. Scalability, bottlenecks, technical debt.',
  SAGE: 'Deep thinker. LLM reasoning, complex analysis, multi-perspective.',
  SCHOLAR: 'Learning tracker. Q-table, feedback loops, knowledge graphs.',
  SCOUT: 'Explorer. Dependencies, ecosystem, social signals, trends.',
}

export default function DogsDeepDive({ dogs }) {
  const dogEntries = Object.entries(dogs || {})
    .map(([name, score]) => ({ name, score }))
    .sort((a, b) => b.score - a.score)

  return (
    <div className="space-y-6">
      <div className="card mb-6">
        <h2 className="card-header">🔬 Dogs Deep Dive</h2>
        <p className="text-sm text-gray-400">
          11 specialist Dogs that run in parallel consensus. Each brings unique perspective.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {dogEntries.map(({ name, score }) => (
          <div key={name} className="card border-l-4 border-l-green-500">
            <h3 className="font-semibold text-gray-50 mb-2">{name}</h3>
            <p className="text-xs text-gray-400 mb-4">{DOG_DESCRIPTIONS[name] || 'Specialist perspective'}</p>

            <div className="space-y-2">
              <div>
                <div className="text-xs text-gray-400 uppercase">Q-Score</div>
                <div className="text-2xl font-bold text-gray-50">{score?.toFixed(1) || '0.0'}</div>
              </div>

              <div>
                <div className="text-xs text-gray-400 uppercase">Status</div>
                <div className={`text-xs font-medium inline-block px-2 py-1 rounded ${
                  score > 80 ? 'bg-green-900 text-green-200' :
                  score > 60 ? 'bg-blue-900 text-blue-200' :
                  score > 40 ? 'bg-amber-900 text-amber-200' :
                  'bg-red-900 text-red-200'
                }`}>
                  {score > 80 ? 'Excellent' :
                   score > 60 ? 'Good' :
                   score > 40 ? 'Fair' :
                   'Low'}
                </div>
              </div>

              <div className="pt-2">
                <div className="relative h-2 bg-gray-700 rounded-full overflow-hidden">
                  <div
                    className="absolute top-0 left-0 h-full bg-green-500 rounded-full transition-all duration-300"
                    style={{ width: `${Math.min(score / 100 * 100, 100)}%` }}
                  />
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Legend */}
      <div className="card bg-gray-700">
        <h3 className="font-semibold text-gray-50 mb-4">Interpretation</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs">
          <div>
            <div className="inline-block w-3 h-3 bg-green-500 rounded mr-2"></div>
            <span className="text-gray-300">Q &gt; 80: Excellent</span>
          </div>
          <div>
            <div className="inline-block w-3 h-3 bg-blue-500 rounded mr-2"></div>
            <span className="text-gray-300">Q 60-80: Good</span>
          </div>
          <div>
            <div className="inline-block w-3 h-3 bg-amber-500 rounded mr-2"></div>
            <span className="text-gray-300">Q 40-60: Fair</span>
          </div>
          <div>
            <div className="inline-block w-3 h-3 bg-red-500 rounded mr-2"></div>
            <span className="text-gray-300">Q &lt; 40: Low</span>
          </div>
        </div>
      </div>

      {/* Principles */}
      <div className="card bg-purple-900 border-purple-700">
        <p className="text-sm text-purple-200">
          <span className="font-semibold">Byzantine Consensus</span><br/>
          11 Dogs vote independently. φ-weighted geometric mean produces final Q-Score. Maximum confidence 61.8% (φ⁻¹) = humility baked in.
        </p>
      </div>
    </div>
  )
}
