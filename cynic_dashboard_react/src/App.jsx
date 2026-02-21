import React, { useState, useEffect } from 'react'
import DogVoting from './components/DogVoting'
import JudgmentDisplay from './components/JudgmentDisplay'
import DecisionTheater from './components/DecisionTheater'
import LearningAnalytics from './components/LearningAnalytics'
import DogsDeepDive from './components/DogsDeepDive'

export default function App() {
  const [activeTab, setActiveTab] = useState('voting')
  const [judgment, setJudgment] = useState(null)
  const [dogs, setDogs] = useState([])
  const [stats, setStats] = useState(null)
  const [connected, setConnected] = useState(false)

  useEffect(() => {
    const connectWebSocket = () => {
      const ws = new WebSocket('ws://localhost:8000/ws/stream')

      ws.onopen = () => {
        setConnected(true)
        console.log('Connected to CYNIC kernel ws/stream')
      }

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data)

          // Handle JUDGMENT_CREATED events from backend
          if (data.type === 'JUDGMENT_CREATED' && data.payload) {
            const judgment = data.payload
            setJudgment(judgment)
            if (judgment.dogs_votes) {
              setDogs(judgment.dogs_votes)
            }
          }
          // Handle ping messages (keepalive)
          else if (data.type === 'ping') {
            // Respond to keepalive ping
            ws.send(JSON.stringify({ type: 'pong' }))
          }
          // Handle initial connection confirmation
          else if (data.type === 'connected') {
            console.log('CYNIC kernel ready, φ =', data.phi)
          }
        } catch (err) {
          console.error('WebSocket message parse error:', err)
        }
      }

      ws.onerror = (error) => {
        console.error('WebSocket error:', error)
        setConnected(false)
      }

      ws.onclose = () => {
        setConnected(false)
        // Reconnect after 3 seconds
        setTimeout(connectWebSocket, 3000)
      }

      return ws
    }

    const ws = connectWebSocket()
    return () => ws?.close()
  }, [])

  const tabs = [
    { id: 'voting', label: '🐕 Dogs Voting' },
    { id: 'judgment', label: '⚖️ Judgment' },
    { id: 'decision', label: '🎭 Decision Theater' },
    { id: 'analytics', label: '📊 Learning' },
    { id: 'deepdive', label: '🔬 Deep Dive' }
  ]

  return (
    <div className="min-h-screen bg-gray-900">
      {/* Header */}
      <header className="border-b border-gray-700 bg-gray-800">
        <div className="container-cynic flex justify-between items-center">
          <div className="py-4">
            <h1 className="text-3xl font-bold text-gray-50">
              <span className="text-red-500">CYNIC</span> Dashboard
            </h1>
            <p className="text-sm text-gray-400 mt-1">Living Organism · φ-Bounded Judgment</p>
          </div>
          <div className={`px-4 py-2 rounded-lg text-sm font-medium ${
            connected
              ? 'bg-green-900 text-green-200'
              : 'bg-red-900 text-red-200'
          }`}>
            {connected ? '🟢 Connected' : '🔴 Disconnected'}
          </div>
        </div>
      </header>

      {/* Navigation Tabs */}
      <nav className="border-b border-gray-700 bg-gray-800 sticky top-0 z-10">
        <div className="container-cynic flex gap-1 overflow-x-auto">
          {tabs.map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-4 py-3 font-medium text-sm whitespace-nowrap transition-colors ${
                activeTab === tab.id
                  ? 'border-b-2 border-green-500 text-green-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </nav>

      {/* Main Content */}
      <main className="container-cynic py-8">
        {activeTab === 'voting' && <DogVoting dogs={dogs} />}
        {activeTab === 'judgment' && <JudgmentDisplay judgment={judgment} />}
        {activeTab === 'decision' && <DecisionTheater judgment={judgment} />}
        {activeTab === 'analytics' && <LearningAnalytics stats={stats} />}
        {activeTab === 'deepdive' && <DogsDeepDive dogs={dogs} />}
      </main>

      {/* Footer */}
      <footer className="border-t border-gray-700 bg-gray-800 mt-12">
        <div className="container-cynic py-4 text-center text-xs text-gray-500">
          <p>*sniff* φ-bounded at 61.8% • Confidence: {judgment?.confidence ? (judgment.confidence * 100).toFixed(1) : '—'}% • Judgment Count: {stats?.judgment_count || 0}</p>
        </div>
      </footer>
    </div>
  )
}
