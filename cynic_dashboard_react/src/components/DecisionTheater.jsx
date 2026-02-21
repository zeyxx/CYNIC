import React, { useState, useEffect } from 'react'
import client from '../api/client'

export default function DecisionTheater({ judgment }) {
  const [proposedActions, setProposedActions] = useState([])
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    const fetchActions = async () => {
      try {
        setLoading(true)
        // Try to fetch proposed actions — endpoint may not exist yet (mock data instead)
        try {
          const response = await client.get('/actions')
          setProposedActions(response.data || [])
        } catch (err) {
          if (err.response?.status === 404) {
            // Endpoint not implemented — show mock actions for now
            setProposedActions([
              {
                id: 'action-1',
                action_type: 'INVESTIGATE',
                priority: 'P1',
                reasoning: 'High complexity spike detected in core consensus logic',
                status: 'pending'
              },
              {
                id: 'action-2',
                action_type: 'REFACTOR',
                priority: 'P2',
                reasoning: 'QTable state management could benefit from consolidation',
                status: 'pending'
              }
            ])
          } else {
            throw err
          }
        }
      } catch (err) {
        console.error('Failed to fetch actions:', err)
      } finally {
        setLoading(false)
      }
    }

    const interval = setInterval(fetchActions, 5000)
    fetchActions()

    return () => clearInterval(interval)
  }, [])

  const handleApprove = async (actionId) => {
    try {
      await client.post(`/actions/${actionId}/accept`)
      setProposedActions(prev => prev.map(a =>
        a.id === actionId ? { ...a, status: 'executing' } : a
      ))
    } catch (err) {
      if (err.response?.status === 404) {
        // Fallback: just update UI
        setProposedActions(prev => prev.map(a =>
          a.id === actionId ? { ...a, status: 'executing' } : a
        ))
      } else {
        console.error('Failed to approve action:', err)
      }
    }
  }

  const handleReject = async (actionId) => {
    try {
      await client.post(`/actions/${actionId}/reject`)
      setProposedActions(prev => prev.map(a =>
        a.id === actionId ? { ...a, status: 'rejected' } : a
      ))
    } catch (err) {
      if (err.response?.status === 404) {
        // Fallback: just update UI
        setProposedActions(prev => prev.map(a =>
          a.id === actionId ? { ...a, status: 'rejected' } : a
        ))
      } else {
        console.error('Failed to reject action:', err)
      }
    }
  }

  return (
    <div className="space-y-6">
      <div className="card">
        <h2 className="card-header">🎭 Decision Theater</h2>
        <p className="text-sm text-gray-400 mb-6">
          Proposed actions from CYNIC's judgments. Approve to execute, reject to provide feedback.
        </p>

        {loading && !proposedActions.length ? (
          <p className="text-gray-400 text-center py-8">Loading actions...</p>
        ) : proposedActions.length === 0 ? (
          <p className="text-gray-400 text-center py-8">No proposed actions</p>
        ) : (
          <div className="space-y-4">
            {proposedActions.map(action => (
              <div key={action.id} className="border border-gray-700 rounded-lg p-4 hover:border-gray-600 transition-colors">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <h3 className="font-semibold text-gray-50">{action.action_type}</h3>
                    <p className="text-xs text-gray-400 mt-1">
                      Priority: <span className="font-medium">{action.priority || 'P2'}</span>
                    </p>
                  </div>
                  <span className={`badge ${
                    action.status === 'pending' ? 'badge-wag' :
                    action.status === 'executing' ? 'badge-howl' :
                    action.status === 'rejected' ? 'badge-bark' :
                    'badge-growl'
                  }`}>
                    {action.status}
                  </span>
                </div>

                {action.reasoning && (
                  <p className="text-sm text-gray-400 mb-3 bg-gray-700 rounded p-2">
                    {action.reasoning}
                  </p>
                )}

                {action.status === 'pending' && (
                  <div className="flex gap-3 mt-4">
                    <button
                      onClick={() => handleApprove(action.id)}
                      className="button-primary text-sm"
                    >
                      ✓ Approve
                    </button>
                    <button
                      onClick={() => handleReject(action.id)}
                      className="button-danger text-sm"
                    >
                      ✗ Reject
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Info Box */}
      <div className="card bg-orange-900 border-orange-700">
        <p className="text-sm text-orange-200">
          <span className="font-semibold">Actions Close The Loop</span><br/>
          Each human decision (approve/reject) teaches CYNIC via the Q-Learning system, improving future judgments.
        </p>
      </div>
    </div>
  )
}
