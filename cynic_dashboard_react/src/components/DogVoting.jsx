import React, { useMemo } from 'react'

const DOG_COLORS = {
  ANALYST: '#8B5CF6',
  ARCHITECT: '#3B82F6',
  CARTOGRAPHER: '#06B6D4',
  CYNIC: '#EF4444',
  DEPLOYER: '#10B981',
  GUARDIAN: '#F59E0B',
  JANITOR: '#6366F1',
  ORACLE: '#EC4899',
  SAGE: '#F97316',
  SCHOLAR: '#8B5CF6',
  SCOUT: '#14B8A6',
}

const DOG_NAMES = Object.keys(DOG_COLORS)

export default function DogVoting({ dogs = {} }) {
  const dogScores = useMemo(() => {
    if (!dogs || typeof dogs !== 'object') return []

    return DOG_NAMES.map(name => ({
      name,
      score: dogs[name] !== undefined ? dogs[name] : 0,
      percentage: (Math.max(dogs[name] || 0, 0) / 100) * 100,
      color: DOG_COLORS[name]
    })).sort((a, b) => b.score - a.score)
  }, [dogs])

  const avgScore = useMemo(() => {
    if (dogScores.length === 0) return 0
    return (dogScores.reduce((sum, d) => sum + d.score, 0) / dogScores.length).toFixed(1)
  }, [dogScores])

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
      {/* Main Voting Display */}
      <div className="lg:col-span-2">
        <div className="card">
          <h2 className="card-header">🐕 11 Dogs Consensus Voting</h2>
          <div className="space-y-6">
            {dogScores.map((dog) => (
              <div key={dog.name} className="flex items-center gap-4">
                <div className="w-24 flex-shrink-0">
                  <div className="dog-name">{dog.name}</div>
                  <div className="text-xs text-gray-400 mt-0.5">Q: {dog.score.toFixed(1)}</div>
                </div>

                <div className="flex-1">
                  <div className="relative h-6 bg-gray-700 rounded-full overflow-hidden">
                    <div
                      className="dog-bar absolute top-0 left-0 h-full transition-all duration-300"
                      style={{
                        width: `${dog.percentage}%`,
                        backgroundColor: dog.color,
                        opacity: 0.8
                      }}
                    />
                    <div className="absolute inset-0 flex items-center justify-end pr-2">
                      <span className="text-xs font-medium text-gray-200">
                        {dog.percentage.toFixed(0)}%
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Statistics Panel */}
      <div className="space-y-4">
        <div className="card">
          <div className="stat-block">
            <div className="stat-label">Average Score</div>
            <div className="stat-value">{avgScore}</div>
          </div>
        </div>

        <div className="card">
          <div className="stat-block">
            <div className="stat-label">Active Dogs</div>
            <div className="stat-value">{dogScores.filter(d => d.score > 0).length}/11</div>
          </div>
        </div>

        <div className="card">
          <div className="stat-block">
            <div className="stat-label">Max Q-Score</div>
            <div className="stat-value">{dogScores[0]?.score.toFixed(1) || '—'}</div>
          </div>
        </div>

        <div className="card">
          <div className="stat-block">
            <div className="stat-label">Min Q-Score</div>
            <div className="stat-value">{dogScores[dogScores.length - 1]?.score.toFixed(1) || '—'}</div>
          </div>
        </div>

        <div className="card bg-blue-900 border-blue-700">
          <p className="text-xs text-blue-200">
            <span className="font-semibold">φ-Weighted Consensus</span><br/>
            Geometric mean of 11 Dogs' scores, max confidence 61.8%
          </p>
        </div>
      </div>
    </div>
  )
}
