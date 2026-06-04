import { useState, useEffect, useRef, useCallback } from 'react'
import './App.css'

const API = 'https://api.talaria.build'

const KIND_COLOR = {
  Howl:'#4ade80', Wag:'#86efac', Growl:'#fbbf24',
  Bark:'#fb923c', Epoche:'#a78bfa'
}
const KIND_BG = {
  Howl:'rgba(74,222,128,0.18)', Wag:'rgba(134,239,172,0.12)',
  Growl:'rgba(251,191,36,0.14)', Bark:'rgba(251,146,60,0.15)',
  Epoche:'rgba(167,139,250,0.14)'
}

const CHESS_FRAMES = [
  ['♜','·','♝','♛','♚','♝','·','♜'],
  ['♟','♟','♟','·','·','♟','♟','♟'],
  ['·','·','·','·','♟','·','·','·'],
  ['·','·','·','♟','♙','·','·','·'],
  ['·','·','·','♙','·','·','·','·'],
  ['·','♘','♙','·','·','♙','·','·'],
  ['♙','♙','·','·','·','·','♙','♙'],
  ['♖','·','♗','♕','♔','♗','♘','♖'],
]

function useInterval(fn, ms) {
  const ref = useRef(fn)
  useEffect(() => { ref.current = fn }, [fn])
  useEffect(() => {
    const t = setInterval(() => ref.current(), ms)
    return () => clearInterval(t)
  }, [ms])
}

// ── Verdict Grid (mempool-style) ──────────────────────────────────────────────
function VerdictGrid({ items, totalCount }) {
  const [hovered, setHovered] = useState(null)

  // Fill grid: real verdicts + grey placeholders to show scale
  const GRID_SIZE = 80
  const real = items.slice(0, GRID_SIZE)
  const placeholders = Math.max(0, GRID_SIZE - real.length)

  return (
    <div className="verdict-grid-wrap">
      <div className="verdict-grid-header">
        <span className="micro-label">Live verdict activity</span>
        <span className="verdict-grid-total">{totalCount.toLocaleString()} total</span>
      </div>
      <div className="verdict-grid">
        {real.map((v, i) => (
          <div
            key={v.ts + i}
            className={`vblock vblock-${v.kind?.toLowerCase() ?? 'unknown'} ${i === 0 ? 'vblock-new' : ''}`}
            style={{ background: KIND_BG[v.kind] ?? 'rgba(201,168,76,0.06)', borderColor: KIND_COLOR[v.kind] ?? 'rgba(201,168,76,0.2)' }}
            onMouseEnter={() => setHovered(v)}
            onMouseLeave={() => setHovered(null)}
          >
            {hovered === v && (
              <div className="vblock-tooltip">
                <span style={{ color: KIND_COLOR[v.kind] ?? 'var(--gold)', fontWeight: 700 }}>
                  {v.kind === 'Epoche' ? 'EPOCHÉ' : (v.kind ?? '?').toUpperCase()}
                </span>
                <span>{Math.round(v.q * 100)} / 61.8</span>
                <span style={{ opacity: 0.6 }}>{v.domain}</span>
                <span style={{ opacity: 0.4 }}>{v.ts?.slice(11, 16)}</span>
              </div>
            )}
          </div>
        ))}
        {Array.from({ length: placeholders }).map((_, i) => (
          <div key={`ph-${i}`} className="vblock vblock-placeholder" />
        ))}
      </div>
      <div className="verdict-grid-legend">
        {Object.entries(KIND_COLOR).map(([kind, color]) => (
          <span key={kind} className="legend-item">
            <span className="legend-dot" style={{ background: color }} />
            {kind === 'Epoche' ? 'EPOCHÉ' : kind.toUpperCase()}
          </span>
        ))}
      </div>
    </div>
  )
}

// ── Live dot ──────────────────────────────────────────────────────────────────
function LiveDot({ active }) {
  return <span className={`live-dot ${active ? 'on' : 'off'}`} />
}

// ── Counter ───────────────────────────────────────────────────────────────────
function Counter({ value }) {
  const [d, setD] = useState(0)
  useEffect(() => {
    if (!value) return
    const diff = value - d
    if (!diff) return
    const steps = Math.min(40, Math.abs(diff))
    let i = 0
    const t = setInterval(() => { i++; setD(Math.round(d + diff * i / steps)); if (i >= steps) clearInterval(t) }, 16)
    return () => clearInterval(t)
  }, [value])
  return <>{d.toLocaleString()}</>
}

// ── Chess board ───────────────────────────────────────────────────────────────
function ChessBoard() {
  const [hl, setHl] = useState(null)
  useEffect(() => {
    const t = setInterval(() => setHl([Math.floor(Math.random()*8), Math.floor(Math.random()*8)]), 1800)
    return () => clearInterval(t)
  }, [])
  return (
    <div className="chess-board">
      {CHESS_FRAMES.map((row, r) => (
        <div key={r} className="chess-row">
          {row.map((p, c) => (
            <span key={c} className={`chess-cell ${(r+c)%2===0?'light':'dark'} ${hl&&hl[0]===r&&hl[1]===c?'hl':''} ${p!=='·'?'piece':''}`}>{p}</span>
          ))}
        </div>
      ))}
    </div>
  )
}

// ── Main ──────────────────────────────────────────────────────────────────────
export default function App() {
  const [stats, setStats] = useState(null)
  const [feed, setFeed] = useState([])
  const [pulse, setPulse] = useState(false)
  const [menuOpen, setMenuOpen] = useState(false)

  const fetchStats = useCallback(async () => {
    try { setStats(await (await fetch(`${API}/demo/stats`)).json()) } catch {}
  }, [])

  const fetchFeed = useCallback(async () => {
    try {
      const data = await (await fetch(`${API}/demo/feed`)).json()
      setFeed(prev => {
        if (!data.length) return prev
        const incoming = data.filter(d => !prev.some(p => p.ts === d.ts && p.kind === d.kind))
        if (incoming.length) { setPulse(true); setTimeout(() => setPulse(false), 800) }
        return [...incoming, ...prev].slice(0, 80)
      })
    } catch {}
  }, [])

  useEffect(() => { fetchStats(); fetchFeed() }, [])
  useInterval(fetchStats, 30000)
  useInterval(fetchFeed, 12000)

  const dogs    = stats?.dogs_online ?? 0
  const verdicts = stats?.verdicts_total ?? 0

  return (
    <div className="page">

      {/* Header */}
      <header className="header">
        <a href="/" className="logo" style={{textDecoration:'none'}}>
          <span className="logo-name">Talaria</span>
          <span className="logo-pipe">·</span>
          <span className="logo-sub">Sovereign judgment infrastructure</span>
        </a>
        <button className="menu-btn" onClick={() => setMenuOpen(o => !o)}>{menuOpen ? '×' : '≡'}</button>
        <nav className={`nav ${menuOpen ? 'open' : ''}`}>
          <a href="https://demo.talaria.build" className="nav-link" onClick={() => setMenuOpen(false)}>Judge a proposal</a>
          <a href="https://blitz-and-chill-web.vercel.app/en" target="_blank" rel="noopener noreferrer" className="nav-link" onClick={() => setMenuOpen(false)}>Play chess</a>
          <a href="https://t.me/TalariaBuild" target="_blank" rel="noopener noreferrer" className="nav-link" onClick={() => setMenuOpen(false)}>Telegram</a>
          <a href="https://www.futard.io/launch/D1zRtkZ8c5JajdkSDPvatpALf74KNQZtfjZvmh1sLZjc"
            target="_blank" rel="noopener noreferrer" className="nav-link nav-ico" onClick={() => setMenuOpen(false)}>ICO LIVE ↗</a>
        </nav>
      </header>

      {/* Observatory bar */}
      <div className="obs-bar">
        <LiveDot active={dogs > 0} />
        <span className="obs-text">{dogs > 0 ? `${dogs} dog${dogs>1?'s':''} online` : 'offline'}</span>
        <span className="obs-sep">·</span>
        <span className="obs-num"><Counter value={verdicts} /></span>
        <span className="obs-text"> verdicts rendered</span>
        <span className="obs-sep">·</span>
        <span className="obs-text">0% cloud</span>
        <div className="obs-spacer" />
        <span className={`pulse-dot ${pulse ? 'active' : ''}`} />
        <span className="obs-text" style={{fontSize:'10px',letterSpacing:'0.08em'}}>LIVE</span>
      </div>

      {/* Hero */}
      <section className="hero">
        <h1 className="hero-title">
          Is the proposal <em>sound?</em><br className="hero-br" />
          Are the voters <em>human?</em>
        </h1>
        <p className="hero-sub">Two live products. One trust layer for Solana governance.</p>
        <div className="hero-actions">
          <a href="https://demo.talaria.build" className="btn-primary">Judge a proposal →</a>
          <a href="https://blitz-and-chill-web.vercel.app/en" target="_blank" rel="noopener noreferrer" className="btn-secondary">Play a game of chess</a>
        </div>
      </section>

      {/* Verdict grid — mempool style */}
      <VerdictGrid items={feed} totalCount={verdicts} />

      {/* Products */}
      <section className="two-products">
        <div className="product-panel">
          <div className="product-eyebrow">CYNIC</div>
          <h2 className="product-heading">AI judgment engine</h2>
          <p className="product-body">Independent validators score proposals on 6 axioms. Confidence bounded at φ⁻¹ = 0.618 — the system refuses certainty beyond the evidence. When validators disagree, judgment is suspended (EPOCHÉ) rather than forced.</p>
          <div className="product-facts">
            <span>Rust kernel · sovereign hardware · zero cloud</span>
            <span>Verdicts recorded on Solana devnet</span>
            <span>Sources cited via Wikipedia RAG</span>
          </div>
          <a href="https://demo.talaria.build" className="product-cta">Try CYNIC →</a>
        </div>
        <div className="product-panel product-panel-bc">
          <div className="product-eyebrow">Blitz & Chill</div>
          <h2 className="product-heading">Chess, but social.</h2>
          <p className="product-body">Learn chess, play against friends, climb the leaderboard. Inter-community tournaments, archetypes, puzzles. Mobile-first. And when you beat the bot — you've proven you're human. No captcha. No KYC.</p>
          <div className="product-facts">
            <span>Real-time multiplayer · solo vs AI · tournaments</span>
            <span>Puzzles · archetypes · community tribes</span>
            <span>Mobile-first · free · open-source</span>
          </div>
          <ChessBoard />
          <a href="https://blitz-and-chill-web.vercel.app/en" target="_blank" rel="noopener noreferrer" className="product-cta product-cta-bc">Play now →</a>
        </div>
      </section>

      {/* Metrics */}
      <section className="metrics-section">
        <p className="micro-label">By the numbers</p>
        <div className="metrics-grid">
          <div className="metric-card"><div className="metric-num"><Counter value={verdicts} /></div><div className="metric-lbl">Verdicts <span className="live-tag">live</span></div></div>
          <div className="metric-card"><div className="metric-num">2,045</div><div className="metric-lbl">Tests (CYNIC + B&amp;C)</div></div>
          <div className="metric-card"><div className="metric-num">1,404</div><div className="metric-lbl">Commits combined</div></div>
          <div className="metric-card"><div className="metric-num">0%</div><div className="metric-lbl">Cloud dependency</div></div>
        </div>
      </section>

      {/* ICO */}
      <section className="ico-section">
        <div className="ico-inner">
          <div>
            <p className="ico-label">$TALARIA · MetaDAO Futardio · ICO live</p>
            <h2 className="ico-title">Own a share of the trust layer.</h2>
            <p className="ico-desc">$50K raise · 7 days · Futarchy-governed treasury. Market-approved monthly allowance — not a salary.</p>
          </div>
          <div className="ico-actions">
            <a href="https://www.futard.io/launch/D1zRtkZ8c5JajdkSDPvatpALf74KNQZtfjZvmh1sLZjc" target="_blank" rel="noopener noreferrer" className="btn-primary btn-large">Invest on Futardio ↗</a>
            <a href="https://t.me/TalariaBuild" target="_blank" rel="noopener noreferrer" className="btn-secondary">Join Telegram</a>
            <a href="https://github.com/zeyxx/CYNIC" target="_blank" rel="noopener noreferrer" className="btn-secondary">View the code</a>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="footer">
        <div className="footer-top">
          <div className="footer-brand">
            <span className="logo-name" style={{fontSize:'15px'}}>Talaria</span>
            <span className="footer-motto">— the winged sandals of Hermes. The messenger who never lies.</span>
          </div>
          <div className="footer-links">
            <a href="https://github.com/zeyxx/CYNIC" target="_blank" rel="noopener noreferrer">CYNIC repo</a>
            <a href="https://github.com/Ragnar-no-sleep/blitz-and-chill" target="_blank" rel="noopener noreferrer">B&C repo</a>
            <a href="https://t.me/TalariaBuild" target="_blank" rel="noopener noreferrer">Telegram</a>
            <a href="/tos">Terms</a>
            <a href="https://x.com/TalariaBuild" target="_blank" rel="noopener noreferrer">@TalariaBuild</a>
          </div>
        </div>
      </footer>
    </div>
  )
}
