import { useState } from 'react';
import { ChessJudge } from './components/ChessJudge';
import { VerdictHistory } from './components/VerdictHistory';
import { HealthIndicator } from './components/HealthIndicator';
import './App.css';

type Tab = 'chess' | 'history';

function App() {
  const [tab, setTab] = useState<Tab>('chess');

  return (
    <div style={{ minHeight: '100vh', background: '#0d0d0d', color: '#e0e0e0', fontFamily: 'system-ui, sans-serif' }}>
      <header style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '16px 24px',
        borderBottom: '1px solid #1a1a1a',
        background: '#0a0a0a',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <div>
            <div style={{ fontSize: 22, fontWeight: 900, letterSpacing: 4, color: '#C9A84C', fontFamily: 'monospace' }}>
              CYNIC V2
            </div>
            <div style={{ fontSize: 11, color: '#444', letterSpacing: 2 }}>EPISTEMIC IMMUNE SYSTEM</div>
          </div>
          <div style={{ height: 32, width: 1, background: '#222' }} />
          <HealthIndicator />
        </div>
        <div style={{ display: 'flex', gap: 4 }}>
          {(['chess', 'history'] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              style={{
                padding: '8px 20px',
                borderRadius: 8,
                border: 'none',
                background: tab === t ? '#C9A84C22' : 'transparent',
                color: tab === t ? '#C9A84C' : '#555',
                cursor: 'pointer',
                fontFamily: 'monospace',
                fontSize: 12,
                letterSpacing: 2,
                borderBottom: tab === t ? '2px solid #C9A84C' : '2px solid transparent',
              }}
            >
              {t.toUpperCase()}
            </button>
          ))}
        </div>
        <div style={{ fontSize: 11, color: '#333', fontFamily: 'monospace' }}>
          φ⁻¹ = 0.618034 · max confidence
        </div>
      </header>

      <main style={{ padding: '24px', maxWidth: 1200, margin: '0 auto' }}>
        {tab === 'chess' && (
          <div>
            <div style={{ marginBottom: 20 }}>
              <h2 style={{ margin: 0, fontSize: 16, color: '#C9A84C', fontFamily: 'monospace', letterSpacing: 2 }}>
                ♟ CHESS JUDGMENT
              </h2>
              <p style={{ color: '#555', fontSize: 13, margin: '6px 0 0' }}>
                Make a move. The CYNIC Dogs will evaluate the strategy through 6 philosophical axioms.
              </p>
            </div>
            <ChessJudge />
          </div>
        )}
        {tab === 'history' && (
          <div>
            <div style={{ marginBottom: 20 }}>
              <h2 style={{ margin: 0, fontSize: 16, color: '#C9A84C', fontFamily: 'monospace', letterSpacing: 2 }}>
                VERDICT HISTORY
              </h2>
              <p style={{ color: '#555', fontSize: 13, margin: '6px 0 0' }}>
                Recent evaluations from all Dogs.
              </p>
            </div>
            <VerdictHistory />
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
