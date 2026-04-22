import { useState } from 'react';
import { ChessJudge } from './components/ChessJudge';
import { TokenScreener } from './components/TokenScreener';
import { VerdictHistory } from './components/VerdictHistory';
import { HealthIndicator } from './components/HealthIndicator';
import { KernelSettings } from './components/KernelSettings';
import { LearnedPatterns } from './components/LearnedPatterns';
import './App.css';

type Tab = 'screen' | 'chess' | 'history' | 'patterns';

const TAB_LABELS: Record<Tab, string> = {
  screen: 'SCREEN',
  chess: 'CHESS',
  history: 'HISTORY',
  patterns: 'PATTERNS',
};

function App() {
  const [tab, setTab] = useState<Tab>('screen');
  const [showSettings, setShowSettings] = useState(false);

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', color: 'var(--text)' }}>
      <header style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 28px',
        height: 56,
        borderBottom: '1px solid var(--border)',
        background: 'var(--surface)',
        position: 'sticky',
        top: 0,
        zIndex: 50,
      }}>
        {/* Logo */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 18, flex: '0 0 auto' }}>
          <div>
            <div style={{
              fontFamily: 'var(--font-display-deco)',
              fontSize: 17,
              fontWeight: 700,
              letterSpacing: 6,
              color: 'var(--gold)',
              lineHeight: 1,
            }}>
              CYNIC
            </div>
            <div style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 8,
              color: 'var(--text-muted)',
              letterSpacing: 3,
              marginTop: 3,
              textTransform: 'uppercase',
            }}>
              Epistemic Immune System
            </div>
          </div>
          <div style={{ width: 1, height: 24, background: 'var(--border-bright)' }} />
          <HealthIndicator />
        </div>

        {/* Tabs — center */}
        <nav style={{ display: 'flex', gap: 0 }}>
          {(Object.keys(TAB_LABELS) as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              style={{
                padding: '0 18px',
                height: 56,
                border: 'none',
                borderBottom: tab === t ? '2px solid var(--gold)' : '2px solid transparent',
                background: 'transparent',
                color: tab === t ? 'var(--gold)' : 'var(--text-dim)',
                cursor: 'pointer',
                fontFamily: 'var(--font-mono)',
                fontSize: 11,
                letterSpacing: 2,
                fontWeight: tab === t ? 500 : 400,
                transition: 'color 0.15s, border-color 0.15s',
              }}
            >
              {TAB_LABELS[t]}
            </button>
          ))}
        </nav>

        {/* Right — phi constant + settings */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 14, flex: '0 0 auto' }}>
          <span style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            color: 'var(--text-muted)',
            letterSpacing: 1,
            userSelect: 'none',
          }}>
            φ⁻¹ = 0.618034
          </span>
          <button
            onClick={() => setShowSettings(true)}
            title="Kernel settings"
            style={{
              background: 'transparent',
              border: '1px solid var(--border-bright)',
              borderRadius: 6,
              color: 'var(--text-dim)',
              cursor: 'pointer',
              padding: '5px 9px',
              fontSize: 13,
              lineHeight: 1,
              transition: 'border-color 0.15s, color 0.15s',
            }}
            onMouseEnter={e => {
              (e.target as HTMLButtonElement).style.borderColor = 'var(--gold-dim)';
              (e.target as HTMLButtonElement).style.color = 'var(--gold)';
            }}
            onMouseLeave={e => {
              (e.target as HTMLButtonElement).style.borderColor = 'var(--border-bright)';
              (e.target as HTMLButtonElement).style.color = 'var(--text-dim)';
            }}
          >
            ⚙
          </button>
        </div>
      </header>

      <main style={{ padding: '28px 24px', maxWidth: 1200, margin: '0 auto' }}>
        {tab === 'screen' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="TOKEN SCREENER"
              sub="Paste a Solana address. Independent Dogs deliberate under φ-bounded doubt."
            />
            <TokenScreener />
          </div>
        )}
        {tab === 'chess' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="♟ CHESS JUDGMENT"
              sub="Each move evaluated through 6 philosophical axioms."
            />
            <ChessJudge />
          </div>
        )}
        {tab === 'history' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="VERDICT HISTORY"
              sub="Recent judgments from all Dogs."
            />
            <VerdictHistory />
          </div>
        )}
        {tab === 'patterns' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="CRYSTALLIZED PATTERNS"
              sub="Knowledge crystallized through multi-agent consensus."
            />
            <LearnedPatterns />
          </div>
        )}
      </main>

      {showSettings && <KernelSettings onClose={() => setShowSettings(false)} />}
    </div>
  );
}

function PageHeading({ title, sub }: { title: string; sub: string }) {
  return (
    <div style={{ marginBottom: 24 }}>
      <h2 style={{
        margin: 0,
        fontFamily: 'var(--font-display)',
        fontSize: 13,
        fontWeight: 600,
        color: 'var(--gold)',
        letterSpacing: 3,
        textTransform: 'uppercase',
      }}>
        {title}
      </h2>
      <p style={{
        color: 'var(--text-dim)',
        fontSize: 11,
        margin: '6px 0 0',
        fontFamily: 'var(--font-mono)',
        letterSpacing: 0.5,
      }}>
        {sub}
      </p>
    </div>
  );
}

export default App;
