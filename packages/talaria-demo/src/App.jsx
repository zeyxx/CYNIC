import { useState, useEffect } from 'react'
import { ChainExplorer } from './ChainExplorer'
import { MetricsPanel } from './MetricsPanel'
import { fetchJson } from './api'
import './App.css'


const LANG_DEFAULT = navigator.language?.startsWith('fr') ? 'fr' : 'en'

const T = {
  en: {
    logo_sub: 'Judgment Engine',
    nav_judge: 'Judge',
    nav_explorer: 'Explorer',
    title: 'Is the proposal',
    title_em: 'sound?',
    desc: 'Paste any governance proposal, motion, or claim. CYNIC scores it on 6 axioms using independent validators. Sources are cited. Results are recorded on Solana.',
    phi_text: 'Confidence bounded at φ⁻¹ = 0.618 — the golden ratio inverse.',
    phi_tooltip: 'φ⁻¹ (phi inverse) = 0.618. CYNIC cannot score above this threshold regardless of input. When validators disagree beyond the threshold, judgment is suspended — EPOCHÉ — rather than forced. This is an architectural constraint, not a guideline.',
    input_label: 'Proposal or claim',
    placeholder: 'Paste a governance proposal, motion, or any claim.\n\nExamples:\n— "Should we hire X as adviser? Scope: ... Compensation: ..."\n— "Allocate $50K from treasury for Y. Success metric: ..."\n— Any claim you want evaluated on 6 axes.',
    judge: 'Judge',
    judging: 'Judging',
    rate: '10 per hour · free · no account',
    ex_label: 'Examples',
    ex_hanson: 'MetaDAO × Robin Hanson',
    steps: ['Analyzing proposal', 'Consulting sources', 'Dogs deliberating', 'Recording on Solana'],
    axioms_title: 'Axiom evaluation',
    phi_ceiling: 'Scores are 0–100 where 61.8 is the hard ceiling (φ⁻¹) — enforced in code, not prompt.',
    reasoning_title: 'Dog reasoning',
    sources_title: 'Sources consulted',
    sources_note: 'These facts were injected into Dog prompts before judgment.',
    onchain: 'Recorded on Solana ↗',
    history_title: 'Session',
    no_data: 'No data stored',
    epoche_title: 'Judgment suspended',
    lang_other: 'FR',
    verdict_desc: {
      Howl:   'Strong signal. The Dogs align and evidence supports the claim.',
      Wag:    'Positive. More evidence for than against, no critical flags.',
      Growl:  'Skeptical. Concerns were raised — review the reasoning before acting.',
      Bark:   'Rejected. The Dogs found critical issues. Do not ignore.',
      Epoche: 'No verdict. The Dogs disagree too much to decide. The system refuses to force a conclusion.',
    },
    verdict_human: {
      Howl:   'This rarely happens. CYNIC almost never reaches its ceiling.',
      Wag:    null,
      Growl:  'Most proposals land here. Skepticism is the default.',
      Bark:   'Something is structurally wrong with this proposal.',
      Epoche: 'Honest uncertainty is better than false confidence.',
    },
  },
  fr: {
    logo_sub: 'Moteur de jugement',
    nav_judge: 'Juger',
    nav_explorer: 'Explorateur',
    title: 'La proposition est-elle',
    title_em: 'fondée ?',
    desc: 'Collez n\'importe quelle proposition de gouvernance, motion ou affirmation. CYNIC la note sur 6 axiomes avec des validateurs indépendants. Sources citées. Résultats enregistrés sur Solana.',
    phi_text: 'Confiance bornée à φ⁻¹ = 0.618 — l\'inverse du nombre d\'or.',
    phi_tooltip: 'φ⁻¹ = 0.618. CYNIC ne peut pas scorer au-delà de ce seuil, quelle que soit l\'entrée. Quand les validateurs sont trop en désaccord, le jugement est suspendu — EPOCHÉ — plutôt que forcé. C\'est une contrainte architecturale, pas une consigne.',
    input_label: 'Proposition ou affirmation',
    placeholder: 'Collez une proposition de gouvernance, motion ou affirmation.\n\nExemples :\n— "Recruter X comme conseiller ? Périmètre : ... Rémunération : ..."\n— "Allouer 50K$ de la trésorerie pour Y. Indicateur de succès : ..."\n— Toute affirmation à évaluer sur 6 axes.',
    judge: 'Juger',
    judging: 'Jugement',
    rate: '10 par heure · gratuit · sans compte',
    ex_label: 'Exemples',
    ex_hanson: 'MetaDAO × Robin Hanson',
    steps: ['Analyse de la proposition', 'Consultation des sources', 'Dogs en délibération', 'Enregistrement sur Solana'],
    axioms_title: 'Évaluation des axiomes',
    phi_ceiling: 'Scores de 0 à 100, plafond dur à 61.8 (φ⁻¹) — imposé par le code, pas par le prompt.',
    reasoning_title: 'Raisonnement du Dog',
    sources_title: 'Sources consultées',
    sources_note: 'Ces faits ont été injectés dans les prompts des Dogs avant le jugement.',
    onchain: 'Enregistré sur Solana ↗',
    history_title: 'Session',
    no_data: 'Aucune donnée stockée',
    epoche_title: 'Jugement suspendu',
    lang_other: 'EN',
    verdict_desc: {
      Howl:   'Signal fort. Les Dogs s\'alignent, les preuves soutiennent la proposition.',
      Wag:    'Positif. Plus d\'éléments pour que contre, pas de drapeaux critiques.',
      Growl:  'Sceptique. Des réserves ont été émises — lisez le raisonnement avant d\'agir.',
      Bark:   'Rejeté. Les Dogs ont trouvé des problèmes critiques. Ne pas ignorer.',
      Epoche: 'Pas de verdict. Les Dogs sont trop en désaccord. Le système refuse de forcer une conclusion.',
    },
    verdict_human: {
      Howl:   'C\'est rare. CYNIC n\'atteint presque jamais son plafond.',
      Wag:    null,
      Growl:  'La plupart des propositions atterrissent ici. Le scepticisme est la valeur par défaut.',
      Bark:   'Quelque chose est structurellement problématique dans cette proposition.',
      Epoche: 'L\'incertitude honnête vaut mieux que la fausse confiance.',
    },
  },
}

const VERDICT_COLORS = {
  Howl:   'var(--howl)',
  Wag:    'var(--wag)',
  Growl:  'var(--growl)',
  Bark:   'var(--bark)',
  Epoche: 'var(--epoche)',
}

const AXIOMS = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'sovereignty']
const PHI_MAX = 0.618
// Score display: multiply by 100 so 0.334 → 33, max 0.618 → 61.8
const toDisplay = v => Math.round(v * 100)
const STEP_MS = [8000, 25000, 45000]

const EXAMPLE_HANSON = 'Should MetaDAO hire Robin Hanson as adviser? Hanson invented futarchy in 1999. MetaDAO is the first large-scale on-chain implementation. Scope: mechanism design advice and co-authorship of research posts. Compensation: 20.9 META tokens (~$24,000) over 24 months.'

// ── Components ──────────────────────────────────────────────────────────

function PhiLine({ t }) {
  const [show, setShow] = useState(false)
  return (
    <div className="phi-line"
      onMouseEnter={() => setShow(true)}
      onMouseLeave={() => setShow(false)}>
      <span className="phi-glyph">φ</span>
      <span>{t.phi_text}</span>
      <span className="phi-q">?</span>
      {show && <div className="phi-tooltip">{t.phi_tooltip}</div>}
    </div>
  )
}

function LoadingPanel({ t, startTime }) {
  const [step, setStep] = useState(0)
  useEffect(() => {
    const timers = STEP_MS.map((d, i) => setTimeout(() => setStep(i + 1), d))
    return () => timers.forEach(clearTimeout)
  }, [startTime])
  return (
    <div className="loading-panel">
      {t.steps.map((s, i) => (
        <div key={i} className={`loading-step ${i < step ? 'done' : i === step ? 'active' : 'pending'}`}>
          <span className="step-marker">{i < step ? '✓' : i === step ? '›' : '·'}</span>
          <span>{s}{i === step ? '…' : ''}</span>
        </div>
      ))}
    </div>
  )
}

function AxiomBar({ name, score, rawScore }) {
  const pct = Math.min((score / PHI_MAX) * 100, 100)
  const capped = rawScore != null && rawScore > PHI_MAX + 0.001
  return (
    <div className="axiom-row">
      <span className="axiom-name">{name}</span>
      <div className="axiom-track">
        <div className="axiom-fill" style={{ width: `${pct}%` }} />
        <div className="phi-marker" />
      </div>
      <div className="axiom-values">
        <span className="axiom-val">{toDisplay(score)}</span>
        {capped && <span className="axiom-raw">↓{toDisplay(rawScore)}</span>}
      </div>
    </div>
  )
}

function buildShareText(result, content, lang) {
  const verdict = result.verdict
  const q = result.q_score?.total ?? 0
  const preview = content.trim().slice(0, 60) + (content.length > 60 ? '…' : '')
  const llm = (result.dog_scores ?? []).find(d => d.dog_id !== 'deterministic-dog')
  const topReason = llm
    ? Object.values(llm.reasoning ?? {}).find(r => r && r.length > 10)
    : null

  const score = `${Math.round(q * 100)} / 61.8`
  const templates = {
    en: {
      Epoche: `CYNIC suspended judgment on: "${preview}"\n\nEPOCHÉ — Dogs disagreed past the ceiling. No forced verdict.${topReason ? `\n\n"${topReason.slice(0, 80)}"` : ''}\n\nTry your own →`,
      Bark:   `CYNIC barked on: "${preview}"\n\nVerdict: BARK · ${score}${topReason ? `\n\n"${topReason.slice(0, 80)}"` : ''}\n\nTry your own →`,
      Growl:  `CYNIC growled on: "${preview}"\n\nVerdict: GROWL · ${score}${topReason ? `\n\n"${topReason.slice(0, 80)}"` : ''}\n\nTry your own →`,
      Wag:    `CYNIC approved: "${preview}"\n\nVerdict: WAG · ${score}\n\nTry your own →`,
      Howl:   `CYNIC strongly approved: "${preview}"\n\nVerdict: HOWL · ${score}\n\nTry your own →`,
    },
    fr: {
      Epoche: `CYNIC a suspendu le jugement sur : "${preview}"\n\nEPOCHÉ — Dogs en désaccord au-delà du plafond.${topReason ? `\n\n"${topReason.slice(0, 80)}"` : ''}\n\nTestez le vôtre →`,
      Bark:   `CYNIC a rejeté : "${preview}"\n\nVerdict : BARK · ${score}\n\nTestez le vôtre →`,
      Growl:  `CYNIC a émis des réserves sur : "${preview}"\n\nVerdict : GROWL · ${score}\n\nTestez le vôtre →`,
      Wag:    `CYNIC a approuvé : "${preview}"\n\nVerdict : WAG · ${score}\n\nTestez le vôtre →`,
      Howl:   `CYNIC a fortement approuvé : "${preview}"\n\nVerdict : HOWL · ${score}\n\nTestez le vôtre →`,
    },
  }
  return (templates[lang]?.[verdict] ?? templates.en[verdict] ?? '') + ' demo.talaria.build'
}

function ShareButton({ result, content, lang }) {
  const [copied, setCopied] = useState(false)
  const text = buildShareText(result, content, lang)

  // Use share_url for proper card embed if available, else fall back to demo URL
  const shareUrl = result.share_url ?? 'https://demo.talaria.build'
  const tweetUrl = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}&url=${encodeURIComponent(shareUrl)}`

  function copyText() {
    navigator.clipboard.writeText(text + '\n' + shareUrl).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }

  return (
    <div className="share-row">
      <a href={tweetUrl} target="_blank" rel="noopener noreferrer" className="share-btn-x">
        Share on X ↗
      </a>
      <button className="share-btn-copy" onClick={copyText}>
        {copied ? 'Copied ✓' : 'Copy text'}
      </button>
    </div>
  )
}

function VerdictDoc({ result, lang, content }) {
  const t = T[lang]
  const verdict = result.verdict
  const isEpoche = verdict === 'Epoche'
  const color = VERDICT_COLORS[verdict] ?? 'var(--gold)'
  const total = result.q_score?.total ?? 0
  const scores = result.q_score ?? {}
  const llmDogs = (result.dog_scores ?? []).filter(d => d.dog_id !== 'deterministic-dog')

  const rawMap = {}
  for (const dog of result.dog_scores ?? []) {
    for (const ax of AXIOMS) {
      const r = dog[`raw_${ax}`]
      if (r != null && (rawMap[ax] == null || r > rawMap[ax])) rawMap[ax] = r
    }
  }

  const label = isEpoche ? 'EPOCHÉ' : verdict.toUpperCase()
  const desc = t.verdict_desc[verdict] ?? ''
  const humanNote = t.verdict_human?.[verdict] ?? null

  // Check if any axiom was raw-capped
  const anyCapped = Object.values(rawMap).some(raw => raw > PHI_MAX + 0.001)

  return (
    <div className="verdict-doc">
      {/* Header */}
      <div className="verdict-header">
        <div className="verdict-kind-block">
          <div className="verdict-kind" style={{ color }}>{label}</div>
          <div className="verdict-desc-line">{desc}</div>
          {humanNote && <div className="verdict-human-note">{humanNote}</div>}
        </div>
        <div className="verdict-meta">
          <div className="verdict-score-block">
            <span className="verdict-score-num" style={{ color }}>{toDisplay(total)}</span>
            <span className="verdict-score-denom"> / 61.8</span>
          </div>
          <div className="verdict-score-label">max possible</div>
          {anyCapped && <div className="verdict-capped-note">Some scores were capped at 61.8</div>}
          <div className="verdict-dogs">{result.dogs_used}</div>
        </div>
      </div>

      {/* EPOCHÉ notice */}
      {isEpoche && (
        <div className="epoche-notice">
          <div className="epoche-notice-title">{t.epoche_title}</div>
          <div className="epoche-notice-text">
            {lang === 'fr'
              ? `Les Dogs sont en désaccord au-delà du seuil φ⁻¹. Axiome le plus disputé : ${result.anomaly_axiom ?? '?'} (Δ=${result.max_disagreement?.toFixed(3) ?? '?'}). Aucun verdict forcé — le système préfère l'incertitude honnête à la certitude fausse.`
              : `Dogs disagree past the φ⁻¹ threshold. Most disputed: ${result.anomaly_axiom ?? '?'} (Δ=${result.max_disagreement?.toFixed(3) ?? '?'}). No verdict forced — the system prefers honest uncertainty over false certainty.`
            }
          </div>
        </div>
      )}

      {/* Axioms */}
      <div className="axiom-section">
        <div className="axiom-header">
          <span className="axiom-section-title">{t.axioms_title}</span>
          <span className="phi-ceiling-declaration">{t.phi_ceiling}</span>
        </div>
        <div className="axiom-grid">
          {AXIOMS.map(ax => (
            <AxiomBar key={ax} name={ax} score={scores[ax] ?? 0} rawScore={rawMap[ax]} />
          ))}
        </div>
      </div>

      {/* Dog reasoning */}
      {llmDogs.map(dog => (
        <div key={dog.dog_id} className="dog-section">
          <div className="dog-section-title">
            {t.reasoning_title}
            <span className="dog-latency">{dog.dog_id} · {dog.latency_ms}ms</span>
          </div>
          <div className="reasoning-list">
            {AXIOMS.map(ax => dog.reasoning?.[ax] ? (
              <div key={ax} className="reasoning-item">
                <span className="reasoning-ax">{ax}</span>
                <span className="reasoning-text">{dog.reasoning[ax]}</span>
              </div>
            ) : null)}
          </div>
        </div>
      ))}

      {/* Sources */}
      {result.sources?.length > 0 && (
        <div className="sources-section">
          <div className="sources-title">{t.sources_title}</div>
          {result.sources.map((s, i) => (
            <div key={i} className="source-item">
              <span className="source-bullet">◈</span>
              <div>
                <a href={s.url} target="_blank" rel="noopener noreferrer" className="source-link">
                  {s.title} ↗
                </a>
                <div className="source-extract">{s.extract}</div>
              </div>
            </div>
          ))}
          <div className="sources-note">{t.sources_note}</div>
        </div>
      )}

      {/* Share */}
      <ShareButton result={result} content={content} lang={lang} />

      {/* Footer */}
      <div className="verdict-footer">
        {result.explorer_url
          ? <a href={result.explorer_url} target="_blank" rel="noopener noreferrer" className="onchain-link">{t.onchain}</a>
          : <span />
        }
        <span className="verdict-id-text">{result.verdict_id?.slice(0, 8)}…</span>
      </div>
    </div>
  )
}

// ── Main ─────────────────────────────────────────────────────────────────

export default function App() {
  const [lang, setLang] = useState(LANG_DEFAULT)
  const t = T[lang]
  const [tab, setTab] = useState('judge')
  const [content, setContent] = useState('')
  const [loading, setLoading] = useState(false)
  const [loadStart, setLoadStart] = useState(null)
  const [result, setResult] = useState(null)
  const [error, setError] = useState(null)
  const [history, setHistory] = useState([])

  async function judge() {
    if (!content.trim()) return
    setLoading(true)
    setLoadStart(Date.now())
    setResult(null)
    setError(null)
    const ctrl = new AbortController()
    const timer = setTimeout(() => ctrl.abort(), 90000)
    try {
      const data = await fetchJson('/judge', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: content.trim() }),
        signal: ctrl.signal,
        timeoutMs: 90000,
      })
      setResult(data)
      setHistory(h => [{
        verdict: data.verdict,
        q: data.q_score?.total ?? 0,
        preview: content.trim().slice(0, 42) + (content.length > 42 ? '…' : ''),
        explorer: data.explorer_url,
      }, ...h].slice(0, 6))
    } catch (e) {
      setError(e.name === 'AbortError' ? 'Judgment timed out (>90s)' : e.message)
    } finally {
      clearTimeout(timer)
      setLoading(false)
      setLoadStart(null)
    }
  }

  return (
    <div className="app">
      {/* Header */}
      <header className="header">
        <a href="https://talaria.build" style={{ textDecoration: 'none' }}>
          <div className="logo-mark">
            <span className="logo-name">Talaria</span>
            <span className="logo-sub">{t.logo_sub}</span>
          </div>
        </a>
        <div className="header-spacer" />
        <span className="devnet-chip">devnet</span>
        <button className="lang-btn" onClick={() => setLang(l => l === 'en' ? 'fr' : 'en')}>
          {t.lang_other}
        </button>
        <a href="https://www.futard.io/launch/D1zRtkZ8c5JajdkSDPvatpALf74KNQZtfjZvmh1sLZjc"
          target="_blank" rel="noopener noreferrer" className="ico-link">
          ICO LIVE ↗
        </a>
      </header>

      <main className="main">
        {/* Navigation */}
        <nav className="sub-nav">
          <button 
            className={`nav-tab ${tab === 'judge' ? 'active' : ''}`}
            onClick={() => setTab('judge')}
          >
            {t.nav_judge}
          </button>
          <button 
            className={`nav-tab ${tab === 'explorer' ? 'active' : ''}`}
            onClick={() => setTab('explorer')}
          >
            {t.nav_explorer}
          </button>
        </nav>

        {tab === 'judge' ? (
          <div className="view-judge">
            {/* Hero */}
            <div className="hero">
              <h1 className="hero-title">
                {t.title} <em>{t.title_em}</em>
              </h1>
              <p className="hero-desc">{t.desc}</p>
              <PhiLine t={t} />
            </div>

            {/* Input */}
            <div className="input-card">
              <div className="input-label">{t.input_label}</div>
              <textarea
                className="proposal-input"
                rows={7}
                placeholder={t.placeholder}
                value={content}
                onChange={e => setContent(e.target.value)}
                disabled={loading}
              />
              <div className="input-footer">
                <span className="char-info">{content.length} / 2000</span>
                <span className="rate-info">{t.rate}</span>
                <button className="submit-btn" onClick={judge}
                  disabled={loading || !content.trim() || content.length > 2000}>
                  {loading ? t.judging + '…' : t.judge}
                </button>
              </div>
            </div>

            {/* Examples */}
            <div className="examples">
              <span className="examples-label">{t.ex_label}</span>
              <button className="example-btn" onClick={() => setContent(EXAMPLE_HANSON)}>
                {t.ex_hanson}
              </button>
            </div>

            {/* Loading */}
            {loading && loadStart && <LoadingPanel t={t} startTime={loadStart} />}

            {/* Error */}
            {error && <div className="error-msg">⚠ {error}</div>}

            {/* Verdict */}
            {result && <VerdictDoc result={result} lang={lang} content={content} />}

            {/* History */}
            {history.length > 0 && (
              <div className="history-panel">
                <div className="history-title">{t.history_title}</div>
                <div className="history-list">
                  {history.map((h, i) => (
                    <div key={i} className="history-item">
                      <span className="hist-kind"
                        style={{ color: VERDICT_COLORS[h.verdict] ?? 'var(--gold)' }}>
                        {h.verdict === 'Epoche' ? 'EPOCHÉ' : h.verdict.toUpperCase()}
                      </span>
                      <span className="hist-score">{h.q.toFixed(3)}</span>
                      <span className="hist-preview">{h.preview}</span>
                      {h.explorer && (
                        <a href={h.explorer} target="_blank" rel="noopener noreferrer"
                          className="hist-link">↗</a>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Telemetry Section */}
            <div style={{ marginTop: 80, borderTop: '1px solid var(--border)', paddingTop: 40 }}>
              <div className="section-label" style={{ marginBottom: 24 }}>System Telemetry</div>
              <MetricsPanel lang={lang} />
            </div>
          </div>
        ) : (
          <div className="view-explorer">
            <div className="hero" style={{ marginBottom: 32 }}>
              <h1 className="hero-title" style={{ fontSize: 32 }}>
                {lang === 'fr' ? 'Explorateur de' : 'CYNIC Chain'} <em>Explorer</em>
              </h1>
              <p className="hero-desc">
                {lang === 'fr' 
                  ? 'Audit complet du Proof-of-History. Chaque jugement et observation est haché et scellé dans des blocs d\'état immuables.' 
                  : 'Full audit of Proof-of-History. Every judgment and observation is hashed and sealed into immutable state blocks.'}
              </p>
            </div>
            <ChainExplorer lang={lang} />
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="page-footer">
        <a href="https://github.com/zeyxx/CYNIC" target="_blank" rel="noopener noreferrer">
          github.com/zeyxx/CYNIC
        </a>
        <span className="footer-sep">·</span>
        <a href="https://talaria.build" target="_blank" rel="noopener noreferrer">talaria.build</a>
        <span className="footer-sep">·</span>
        <span>{t.no_data}</span>
      </footer>
    </div>
  )
}
