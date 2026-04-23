import { useState, useEffect, useRef, useCallback } from 'react';
import { judgeAsync, getJudgeStatus } from '../api';
import type { JudgeRequest, AsyncJudgeStatus, DogArrival, Verdict, VerdictKind } from '../types';
import { VERDICT_COLORS, AXIOM_COLORS, AXIOM_ICONS } from '../types';

interface Props {
  request: JudgeRequest | null;
  onComplete?: (verdict: Verdict) => void;
}

type Phase = 'idle' | 'submitting' | 'polling' | 'complete' | 'error';

export function DeliberationView({ request, onComplete }: Props) {
  const [phase, setPhase] = useState<Phase>('idle');
  const [status, setStatus] = useState<AsyncJudgeStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const requestIdRef = useRef<string | null>(null);

  const cleanup = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (!request) return;

    cleanup();
    setPhase('submitting');
    setStatus(null);
    setError(null);

    judgeAsync(request)
      .then((res) => {
        requestIdRef.current = res.request_id;
        setPhase('polling');

        pollRef.current = setInterval(async () => {
          try {
            const s = await getJudgeStatus(res.request_id);
            setStatus(s);
            if (s.status === 'complete') {
              cleanup();
              setPhase('complete');
              if (s.verdict && onComplete) onComplete(s.verdict);
            } else if (s.status === 'failed') {
              cleanup();
              setPhase('error');
              setError(s.error ?? 'Unknown failure');
            }
          } catch (e) {
            cleanup();
            setPhase('error');
            setError(e instanceof Error ? e.message : 'Poll failed');
          }
        }, 2000);
      })
      .catch((e) => {
        setPhase('error');
        setError(e instanceof Error ? e.message : 'Submission failed');
      });

    return cleanup;
  }, [request, cleanup, onComplete]);

  if (phase === 'idle') {
    return (
      <div style={{
        padding: '48px 24px',
        textAlign: 'center',
        border: '1px dashed var(--border)',
        borderRadius: 10,
        background: 'var(--surface)',
      }}>
        <div style={{
          fontFamily: 'var(--font-display)',
          fontSize: 11,
          color: 'var(--text-muted)',
          letterSpacing: 4,
          textTransform: 'uppercase',
          marginBottom: 12,
        }}>
          Dogs Standing By
        </div>
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--text-muted)' }}>
          Submit content to begin deliberation
        </div>
      </div>
    );
  }

  if (phase === 'error') {
    return (
      <div style={{
        padding: 20,
        border: '1px solid #F4433633',
        borderRadius: 8,
        background: '#0d0000',
        fontFamily: 'var(--font-mono)',
        fontSize: 12,
        color: '#F44336',
        letterSpacing: 0.5,
      }}>
        DELIBERATION FAILED — {error}
      </div>
    );
  }

  const dogsTotal = status?.dogs_total ?? 0;
  const arrivals = status?.dogs_arrived ?? [];
  const verdict = status?.verdict ?? null;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <ProgressHeader
        phase={phase}
        arrived={arrivals.length}
        total={dogsTotal}
        verdict={verdict}
      />

      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        {arrivals.map((dog, i) => (
          <DogCard key={dog.dog_id} arrival={dog} index={i} />
        ))}
        {phase === 'polling' && Array.from({ length: Math.max(0, dogsTotal - arrivals.length) }).map((_, i) => (
          <PendingDogSlot key={`pending-${i}`} />
        ))}
      </div>

      {verdict && <FinalVerdict verdict={verdict} />}
    </div>
  );
}

function ProgressHeader({ phase, arrived, total, verdict }: {
  phase: Phase;
  arrived: number;
  total: number;
  verdict: Verdict | null;
}) {
  const progress = total > 0 ? arrived / total : 0;
  const verdictKind = verdict?.verdict as VerdictKind | undefined;
  const color = verdictKind ? VERDICT_COLORS[verdictKind] : 'var(--gold)';

  return (
    <div style={{
      background: 'var(--card)',
      borderRadius: 8,
      padding: '12px 16px',
      border: '1px solid var(--border)',
    }}>
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: 10,
      }}>
        <span style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 11,
          color: 'var(--text-dim)',
          letterSpacing: 1,
        }}>
          {phase === 'submitting' && 'SUBMITTING…'}
          {phase === 'polling' && `DELIBERATING — ${arrived} / ${total} DOGS`}
          {phase === 'complete' && `VERDICT REACHED — ${arrived} / ${total} DOGS`}
        </span>
        {verdict && (
          <span style={{
            fontFamily: 'var(--font-display)',
            fontSize: 12,
            fontWeight: 700,
            color,
            letterSpacing: 3,
          }}>
            {verdict.verdict}
          </span>
        )}
      </div>
      <div style={{
        height: 3,
        borderRadius: 2,
        background: 'var(--border)',
        overflow: 'hidden',
      }}>
        <div style={{
          height: '100%',
          width: `${progress * 100}%`,
          background: color,
          borderRadius: 2,
          transition: 'width 0.6s cubic-bezier(0.4, 0, 0.2, 1)',
        }} />
      </div>
    </div>
  );
}

function DogCard({ arrival, index }: { arrival: DogArrival; index: number }) {
  const [visible, setVisible] = useState(false);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => setVisible(true), index * 80);
    return () => clearTimeout(timer);
  }, [index]);

  const baseStyle: React.CSSProperties = {
    padding: '12px 14px',
    borderRadius: 8,
    opacity: visible ? 1 : 0,
    transform: visible ? 'translateX(0)' : 'translateX(-16px)',
    transition: 'opacity 0.3s ease, transform 0.3s ease',
  };

  if (!arrival.success) {
    return (
      <div style={{
        ...baseStyle,
        background: '#0d0808',
        border: '1px solid #2a1111',
      }}>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: '#F44336' }}>
          {arrival.dog_id} — FAILED: {arrival.error}
        </span>
      </div>
    );
  }

  const score = arrival.score!;
  const axioms = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'sovereignty'] as const;
  const hasReasoning = axioms.some(ax => score.reasoning?.[ax]);

  return (
    <div style={{
      ...baseStyle,
      background: 'var(--card)',
      border: '1px solid var(--border)',
    }}>
      <div
        onClick={() => hasReasoning && setExpanded(e => !e)}
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 10,
          cursor: hasReasoning ? 'pointer' : 'default',
        }}
      >
        <span style={{
          fontFamily: 'var(--font-display)',
          fontSize: 11,
          color: 'var(--gold)',
          fontWeight: 600,
          letterSpacing: 2,
          textTransform: 'uppercase',
        }}>
          {arrival.dog_id}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            color: 'var(--text-muted)',
            letterSpacing: 0.5,
          }}>
            {arrival.arrived_at_ms}ms
          </span>
          {hasReasoning && (
            <span style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: 'var(--text-muted)',
              transition: 'transform 0.2s ease',
              display: 'inline-block',
              transform: expanded ? 'rotate(180deg)' : 'rotate(0deg)',
            }}>▾</span>
          )}
        </div>
      </div>
      <div style={{ display: 'flex', gap: 5, flexWrap: 'wrap' }}>
        {axioms.map((ax) => (
          <AxiomPill key={ax} axiom={ax} value={score[ax]} />
        ))}
      </div>
      {expanded && score.reasoning && (
        <div style={{
          marginTop: 10,
          borderTop: '1px solid var(--border)',
          paddingTop: 10,
          display: 'flex',
          flexDirection: 'column',
          gap: 6,
        }}>
          {axioms
            .filter(ax => score.reasoning[ax])
            .map(ax => (
              <div key={ax} style={{ display: 'flex', gap: 8, alignItems: 'flex-start' }}>
                <span style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 8,
                  color: AXIOM_COLORS[ax],
                  letterSpacing: 1.5,
                  textTransform: 'uppercase',
                  paddingTop: 1,
                  flexShrink: 0,
                  minWidth: 72,
                }}>
                  {AXIOM_ICONS[ax]} {ax}
                </span>
                <span style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 10,
                  color: 'var(--text-dim)',
                  lineHeight: 1.5,
                }}>
                  {score.reasoning[ax]}
                </span>
              </div>
            ))}
        </div>
      )}
    </div>
  );
}

function AxiomPill({ axiom, value }: { axiom: string; value: number }) {
  const color = AXIOM_COLORS[axiom] ?? '#666';
  const icon = AXIOM_ICONS[axiom] ?? '';
  const pct = Math.round(value * 100);

  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 4,
      padding: '3px 8px',
      borderRadius: 4,
      background: `${color}0d`,
      border: `1px solid ${color}22`,
      fontSize: 11,
      fontFamily: 'var(--font-mono)',
    }}>
      <span style={{ fontSize: 10 }}>{icon}</span>
      <span style={{ color, fontWeight: 500 }}>{pct}</span>
    </div>
  );
}

function PendingDogSlot() {
  return (
    <div style={{
      padding: '12px 14px',
      background: 'transparent',
      borderRadius: 8,
      border: '1px dashed var(--border)',
      display: 'flex',
      alignItems: 'center',
      gap: 10,
    }}>
      <div style={{
        width: 6,
        height: 6,
        borderRadius: '50%',
        background: 'var(--text-muted)',
        animation: 'pulse 1.8s ease-in-out infinite',
      }} />
      <span style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--text-muted)',
        letterSpacing: 1,
      }}>
        Awaiting Dog…
      </span>
    </div>
  );
}

function FinalVerdict({ verdict }: { verdict: Verdict }) {
  const kind = verdict.verdict as VerdictKind;
  const color = VERDICT_COLORS[kind];
  const axioms = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'sovereignty'] as const;
  const qPct = (verdict.q_score.total * 100).toFixed(0);

  return (
    <div style={{
      padding: '28px 24px',
      background: 'var(--card)',
      borderRadius: 10,
      border: `1px solid ${color}33`,
      position: 'relative',
      overflow: 'hidden',
      animation: 'verdict-reveal 0.4s cubic-bezier(0.16, 1, 0.3, 1)',
    }}>
      {/* Ambient glow behind verdict */}
      <div style={{
        position: 'absolute',
        top: 0, left: 0, right: 0,
        height: 120,
        background: `radial-gradient(ellipse at 50% -20%, ${color}12 0%, transparent 70%)`,
        pointerEvents: 'none',
      }} />

      {/* Top row: verdict name + Q circle */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'flex-start',
        marginBottom: 24,
        position: 'relative',
      }}>
        <div>
          <div style={{
            fontFamily: 'var(--font-display-deco)',
            fontSize: 40,
            fontWeight: 700,
            color,
            letterSpacing: 6,
            lineHeight: 1,
            marginBottom: 8,
          }}>
            {kind}
          </div>
          <div style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            color: 'var(--text-muted)',
            letterSpacing: 1.5,
          }}>
            {verdict.dogs_used} DOGS · φ⁻¹ = 0.618034 · {verdict.verdict_id?.slice(0, 8)}
          </div>
        </div>

        {/* Q Score circle */}
        <div style={{
          width: 68,
          height: 68,
          borderRadius: '50%',
          border: `2px solid ${color}66`,
          background: `${color}0d`,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
          boxShadow: `0 0 24px ${color}1a`,
        }}>
          <div style={{
            fontFamily: 'var(--font-display)',
            fontSize: 20,
            fontWeight: 700,
            color,
            lineHeight: 1,
          }}>
            {qPct}
          </div>
          <div style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 8,
            color: `${color}88`,
            letterSpacing: 1.5,
            marginTop: 2,
          }}>
            Q·SCORE
          </div>
        </div>
      </div>

      {/* Axiom breakdown */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(3, 1fr)',
        gap: 8,
      }}>
        {axioms.map((ax) => (
          <AxiomBar
            key={ax}
            axiom={ax}
            value={verdict.q_score[ax]}
          />
        ))}
      </div>

      {/* Anomaly banner */}
      {verdict.anomaly_detected && (
        <div style={{
          marginTop: 16,
          padding: '10px 14px',
          background: 'rgba(201, 168, 76, 0.05)',
          border: '1px solid rgba(201, 168, 76, 0.18)',
          borderRadius: 6,
          display: 'flex',
          alignItems: 'center',
          gap: 10,
        }}>
          <span style={{ color: 'var(--gold)', fontSize: 12, flexShrink: 0 }}>◈</span>
          <div>
            <span style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: 'var(--gold)',
              letterSpacing: 2,
            }}>
              ANOMALY
            </span>
            <span style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: 'var(--text-dim)',
              marginLeft: 10,
            }}>
              Dogs disagree on {verdict.anomaly_axiom} — max Δ {verdict.max_disagreement.toFixed(3)}
            </span>
          </div>
        </div>
      )}

      {/* Consensus reasoning */}
      {axioms.some(ax => verdict.reasoning?.[ax]) && (
        <div style={{
          marginTop: 20,
          paddingTop: 16,
          borderTop: '1px solid var(--border)',
        }}>
          <div style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 8,
            color: 'var(--text-muted)',
            letterSpacing: 3,
            textTransform: 'uppercase',
            marginBottom: 12,
          }}>
            Consensus Reasoning
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {axioms
              .filter(ax => verdict.reasoning?.[ax])
              .map(ax => (
                <div key={ax} style={{ display: 'flex', gap: 10, alignItems: 'flex-start' }}>
                  <div style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 8,
                    color: AXIOM_COLORS[ax],
                    letterSpacing: 1.5,
                    textTransform: 'uppercase',
                    paddingTop: 1,
                    flexShrink: 0,
                    minWidth: 80,
                  }}>
                    {AXIOM_ICONS[ax]} {ax}
                  </div>
                  <div style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 11,
                    color: 'var(--text-dim)',
                    lineHeight: 1.6,
                  }}>
                    {verdict.reasoning[ax]}
                  </div>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
}

function AxiomBar({ axiom, value }: { axiom: string; value: number }) {
  const color = AXIOM_COLORS[axiom] ?? '#666';
  const icon = AXIOM_ICONS[axiom] ?? '';
  const fillPct = Math.min(100, (value / 0.618) * 100);

  return (
    <div style={{
      padding: '10px 12px',
      background: 'var(--surface)',
      borderRadius: 6,
      border: '1px solid var(--border)',
    }}>
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: 8,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
          <span style={{ fontSize: 11 }}>{icon}</span>
          <span style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 9,
            color,
            letterSpacing: 1.5,
            textTransform: 'uppercase',
          }}>
            {axiom}
          </span>
        </div>
        <span style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 10,
          color: 'var(--text-dim)',
          fontWeight: 500,
        }}>
          {value.toFixed(3)}
        </span>
      </div>
      <div style={{
        height: 3,
        borderRadius: 2,
        background: 'var(--border)',
        overflow: 'hidden',
      }}>
        <div style={{
          height: '100%',
          width: `${fillPct}%`,
          background: color,
          borderRadius: 2,
          animation: 'bar-fill 0.8s cubic-bezier(0.4, 0, 0.2, 1)',
        }} />
      </div>
    </div>
  );
}
