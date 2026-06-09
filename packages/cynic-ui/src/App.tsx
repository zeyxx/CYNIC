import { useEffect, useMemo, useState } from 'react';
import { BlockExplorer } from './components/BlockExplorer';
import { OracleView } from './components/OracleView';
import { WalletAuthPanel } from './components/WalletAuthPanel';
import { SurfacePanel } from './components/SurfacePanel';
import './App.css';

type Tab = 'timeline' | 'topology' | 'oracle';

const TAB_LABELS: Record<Tab, string> = {
  timeline: 'TIMELINE',
  topology: 'TOPOLOGY',
  oracle: 'ORACLE',
};

function TopologyPanel() {
  const surfaces = [
    {
      label: 'PUBLIC EDGE',
      title: 'Vercel portal',
      body: 'Publishes the operator UI and keeps preview traffic close to the edge.',
      tone: 'is-info',
      chips: ['public', 'preview', 'shared UI'],
    },
    {
      label: 'LOCAL FALLBACK',
      title: 'Containerized portal',
      body: 'Runs the same interface locally or on Tailscale when the edge is unavailable.',
      tone: 'is-success',
      chips: ['fallback relay', 'operator mirror', 'offline-safe'],
    },
    {
      label: 'PRIVATE KERNEL',
      title: 'CYNIC REST bus',
      body: 'Holds the source of truth, sessions, observations, and remediation dispatch.',
      tone: 'is-gold',
      chips: ['private', 'wallet session', 'coordination'],
    },
  ] as const;

  return (
    <SurfacePanel
      eyebrow="TOPOLOGY"
      title="Surface map"
      subtitle="One public edge, one local fallback, one private kernel. The operator uses the same portal on both sides."
    >
      <div className="topology-grid">
        {surfaces.map((surface) => (
          <article key={surface.label} className="surface-card">
            <div className="meta-line">
              <span className={`status-chip ${surface.tone}`}>{surface.label}</span>
            </div>
            <h3>{surface.title}</h3>
            <p>{surface.body}</p>
            <div className="pill-row">
              {surface.chips.map((chip) => (
                <span key={chip} className="pill is-muted">
                  {chip}
                </span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </SurfacePanel>
  );
}

function App() {
  const [tab, setTab] = useState<Tab>('timeline');
  const [isAdmin, setIsAdmin] = useState(() => {
    if (typeof window === 'undefined') return false;
    return window.localStorage.getItem('cynic_admin_mode') === '1';
  });

  useEffect(() => {
    window.localStorage.setItem('cynic_admin_mode', isAdmin ? '1' : '0');
  }, [isAdmin]);

  const heroNotes = useMemo(
    () => [
      { label: 'PUBLIC EDGE', tone: 'is-info' as const },
      { label: 'LOCAL FALLBACK', tone: 'is-success' as const },
      { label: 'PRIVATE KERNEL', tone: 'is-gold' as const },
    ],
    [],
  );

  return (
    <div className="portal-shell">
      <header className="portal-header">
        <div className="brand-mark">
          <div className="brand-name">CYNIC OBSERVATORY</div>
          <div className="brand-sub">Wallet session · Vercel edge · local fallback · private kernel</div>
        </div>

        <div className="header-meta">
          <div className="status-strip">
            {heroNotes.map((chip) => (
              <span key={chip.label} className={`status-chip ${chip.tone}`}>
                {chip.label}
              </span>
            ))}
          </div>
        </div>

        <nav className="portal-nav" aria-label="Portal sections">
          {(Object.keys(TAB_LABELS) as Tab[]).map((t) => (
            <button key={t} onClick={() => setTab(t)} className={`tab-btn ${tab === t ? 'active' : ''}`}>
              {TAB_LABELS[t]}
            </button>
          ))}
          <button
            onClick={() => setIsAdmin((value) => !value)}
            className={`nav-btn ${isAdmin ? 'active' : ''}`}
          >
            ADMIN
          </button>
        </nav>
      </header>

      <main className="portal-main">
        <section className="hero-block">
          <div className="hero-copy-block">
            <span className="hero-kicker">OPERATIONS SURFACE</span>
            <h1 className="hero-title">Measure, review, remediate.</h1>
            <p className="hero-copy">
              The portal is the shared operator surface. Vercel publishes the edge, the local container is the fallback relay, and the kernel stays private behind wallet session auth.
            </p>
          </div>
          <div className="hero-aside">
            <span className="status-chip is-gold">WALLET AUTH</span>
            <span className="status-chip is-success">REMEDIATION READY</span>
          </div>
        </section>

        <WalletAuthPanel />

        {tab === 'timeline' && <BlockExplorer />}
        {tab === 'topology' && <TopologyPanel />}
        {tab === 'oracle' && <OracleView isAdmin={isAdmin} />}
      </main>
    </div>
  );
}

export default App;
