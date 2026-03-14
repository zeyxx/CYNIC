import type { Verdict } from '../types';
import { VERDICT_COLORS, VERDICT_BG, PHI_MAX, AXIOMS } from '../types';
import { AxiomRadar } from './AxiomRadar';

interface Props {
  verdict: Verdict | null;
  loading: boolean;
}

export function VerdictPanel({ verdict, loading }: Props) {
  if (loading) {
    return (
      <div className="verdict-panel verdict-loading">
        <div className="loading-ring" />
        <p>Evaluating…</p>
      </div>
    );
  }

  if (!verdict) {
    return (
      <div className="verdict-panel verdict-empty">
        <div className="empty-icon">φ</div>
        <p>Make a move or submit text<br />to receive judgment</p>
        <p className="phi-note">φ_max = {PHI_MAX.toFixed(6)}</p>
      </div>
    );
  }

  const color = VERDICT_COLORS[verdict.verdict];
  const bg = VERDICT_BG[verdict.verdict];
  const pct = Math.round((verdict.q_score.total / PHI_MAX) * 100);

  return (
    <div className="verdict-panel" style={{ '--verdict-color': color, '--verdict-bg': bg } as React.CSSProperties}>
      {/* Verdict badge */}
      <div className="verdict-badge" style={{ color, background: bg, borderColor: color }}>
        {verdict.verdict.toUpperCase()}
      </div>

      {/* Q-Score */}
      <div className="qscore-block">
        <div className="qscore-value" style={{ color }}>
          {verdict.q_score.total.toFixed(4)}
        </div>
        <div className="qscore-meta">
          <span>{pct}% of φ_max</span>
          <span className="phi-cap">cap {PHI_MAX.toFixed(4)}</span>
        </div>
        <div className="qscore-bar">
          <div className="qscore-bar-fill" style={{ width: `${pct}%`, background: color }} />
          <div className="qscore-bar-phi" />
        </div>
      </div>

      {/* Radar */}
      <div className="radar-block">
        <AxiomRadar qScore={verdict.q_score} color={color} />
      </div>

      {/* Axiom scores */}
      <div className="axioms-block">
        {AXIOMS.map(({ key, label, icon }) => {
          const score = verdict.q_score[key];
          const axiomPct = Math.round((score / PHI_MAX) * 100);
          return (
            <div key={key} className="axiom-row">
              <div className="axiom-header">
                <span className="axiom-icon">{icon}</span>
                <span className="axiom-label">{label}</span>
                <span className="axiom-score" style={{ color }}>{score.toFixed(3)}</span>
                <div className="axiom-bar">
                  <div className="axiom-bar-fill" style={{ width: `${axiomPct}%`, background: color }} />
                </div>
              </div>
              <p className="axiom-reasoning">{verdict.reasoning[key]}</p>
            </div>
          );
        })}
      </div>

      {/* Anomaly */}
      {verdict.anomaly_detected && (
        <div className="anomaly-banner">
          ⚡ Dogs disagree on <strong>{verdict.anomaly_axiom}</strong>
          <span> — discovery signal (Δ {verdict.max_disagreement.toFixed(3)})</span>
        </div>
      )}

      {/* Footer */}
      <div className="verdict-footer">
        <span>evaluated by</span>
        <span className="dogs-used">{verdict.dogs_used.split('+').join(' + ')}</span>
      </div>
    </div>
  );
}
