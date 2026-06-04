import { useState } from 'react';
import { BlockExplorer } from './components/BlockExplorer';
import './App.css'; 

type Tab = 'timeline' | 'topology' | 'oracle';

const TAB_LABELS: Record<Tab, string> = {
  timeline: 'TIMELINE',
  topology: 'TOPOLOGY',
  oracle: 'ORACLE',
};

function App() {
  const [tab, setTab] = useState<Tab>('timeline');

  return (
    <div className="app-container">
      <header className="header">
        <div style={{ color: 'var(--gold)', marginRight: '20px', fontWeight: 'bold' }}>CYNIC OBSERVATORY</div>
        <nav style={{ display: 'flex', gap: '10px' }}>
          {(Object.keys(TAB_LABELS) as Tab[]).map((t) => (
            <button key={t} onClick={() => setTab(t)} className={`nav-btn ${tab === t ? 'active' : ''}`}>
              {TAB_LABELS[t]}
            </button>
          ))}
        </nav>
      </header>
      <main style={{ padding: '20px' }}>
        <h2>{TAB_LABELS[tab]}</h2>
        {tab === 'timeline' && <BlockExplorer />}
        {tab === 'topology' && <div>[TOPOLOGY GRAPH]</div>}
        {tab === 'oracle' && <div>[ORACLE LOGS]</div>}
      </main>
    </div>
  );
}

export default App;
