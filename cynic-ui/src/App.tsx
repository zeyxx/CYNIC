import { useState } from 'react';
import { ChessJudge } from './components/ChessJudge';
import { VerdictHistory } from './components/VerdictHistory';
import { HealthIndicator } from './components/HealthIndicator';
import { KernelSettings } from './components/KernelSettings';
import './App.css';

type Tab = 'chess' | 'history';

function App() {
  const [tab, setTab] = useState<Tab>('chess');
  const [showSettings, setShowSettings] = useState(false);

  return (
    <div style={{ minHeight: '100vh', background: '#0d0d0d', color: '#e0e0e0', fontFamily: 'system-ui, sans-serif' }}>
      <header style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '14px 24px',
        borderBottom: '1px solid #1a1a1a',
        background: '#0a0a0a',
        gap: 16,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16, flex: '0 0 auto' }}>
          <div>
            <div style={{ fontSize: 20, fontWeight: 900, letterSpacing: 4, color: '#C9A84C', fontFamily: 'monospace' }}>
              CYNIC V2
            </div>
            <div style={{ fontSize: 10, color: '#444', letterSpacing: 2 }}>EPISTEMIC IMMUNE SYSTEM</div>
          </div>
          <div style={{ height: 28, width: 1, background: '#222' }} />
          <HealthIndicator />
        </div>

        <div style={{ display: 'flex', gap: 4 }}>
          {(['chess', 'history'] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              style={{
                padding: '7px 18px',
                borderRadius: 7,
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

        <div style={{ display: 'flex', alignItems: 'center', gap: 12, flex: '0 0 auto' }}>
          <span style={{ fontSize: 10, color: '#2a2a2a', fontFamily: 'monospace' }}>
            φ⁻¹ = 0.618034
          </span>
          <button
            onClick={() => setShowSettings(true)}
            title="Kernel settings"
            style={{
              background: '#1a1a1a',
              border: '1px solid #333',
              borderRadius: 6,
              color: '#666',
              cursor: 'pointer',
              padding: '6px 10px',
              fontSize: 14,
              lineHeight: 1,
            }}
          >
            ⚙
          </button>
        </div>
      </header>

      <main style={{ padding: '24px', maxWidth: 1300, margin: '0 auto' }}>
        {tab === 'chess' && (
          <div>
            <div style={{ marginBottom: 16 }}>
              <h2 style={{ margin: 0, fontSize: 15, color: '#C9A84C', fontFamily: 'monospace', letterSpacing: 2 }}>
                ♟ CHESS JUDGMENT
              </h2>
              <p style={{ color: '#444', fontSize: 12, margin: '5px 0 0' }}>
                Cliquez ou glissez les pièces. Chaque coup est évalué par les Dogs CYNIC à travers 6 axiomes philosophiques.
              </p>
            </div>
            <ChessJudge />
          </div>
        )}
        {tab === 'history' && (
          <div>
            <div style={{ marginBottom: 16 }}>
              <h2 style={{ margin: 0, fontSize: 15, color: '#C9A84C', fontFamily: 'monospace', letterSpacing: 2 }}>
                VERDICT HISTORY
              </h2>
              <p style={{ color: '#444', fontSize: 12, margin: '5px 0 0' }}>
                Évaluations récentes de tous les Dogs.
              </p>
            </div>
            <VerdictHistory />
          </div>
        )}
      </main>

      {showSettings && <KernelSettings onClose={() => setShowSettings(false)} />}
    </div>
  );
}

export default App;
