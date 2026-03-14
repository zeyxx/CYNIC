import { useState } from 'react';
import { HealthDot } from './components/HealthDot';
import { ChessTab } from './components/ChessTab';
import { TextTab } from './components/TextTab';
import { VerdictPanel } from './components/VerdictPanel';
import type { Verdict } from './types';
import './App.css';

type Tab = 'chess' | 'text';

function App() {
  const [tab, setTab] = useState<Tab>('chess');
  const [verdict, setVerdict] = useState<Verdict | null>(null);
  const [loading, setLoading] = useState(false);

  const handleTabSwitch = (t: Tab) => {
    setTab(t);
    setVerdict(null);
  };

  return (
    <div className="app">
      <header className="app-header">
        <div className="logo">
          <span className="logo-glyph">♟</span>
          <span className="logo-name">CYNIC</span>
          <span className="logo-tagline">epistemic judge</span>
        </div>

        <nav className="tab-nav">
          <button
            className={`tab-btn ${tab === 'chess' ? 'active' : ''}`}
            onClick={() => handleTabSwitch('chess')}
          >
            Chess
          </button>
          <button
            className={`tab-btn ${tab === 'text' ? 'active' : ''}`}
            onClick={() => handleTabSwitch('text')}
          >
            Text
          </button>
        </nav>

        <HealthDot />
      </header>

      <main className="app-main">
        <section className="left-panel">
          {tab === 'chess'
            ? <ChessTab onVerdict={setVerdict} onLoading={setLoading} />
            : <TextTab onVerdict={setVerdict} onLoading={setLoading} />
          }
        </section>

        <section className="right-panel">
          <VerdictPanel verdict={verdict} loading={loading} />
        </section>
      </main>
    </div>
  );
}

export default App;
