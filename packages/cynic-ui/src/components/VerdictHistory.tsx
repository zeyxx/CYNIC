import { useEffect, useState } from 'react';
import { getVerdicts } from '../api';
import { VERDICT_COLORS } from '../types';
import type { Verdict } from '../types';
import { VerdictDisplay } from './VerdictDisplay';

export function VerdictHistory() {
  const [verdicts, setVerdicts] = useState<Verdict[]>([]);
  const [selected, setSelected] = useState<Verdict | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const data = await getVerdicts();
      setVerdicts(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  if (loading) return <div style={{ color: '#555', padding: 20, textAlign: 'center' }}>Loading history...</div>;
  if (error) return <div style={{ color: '#F44336', padding: 20 }}>⚠ {error}</div>;
  if (verdicts.length === 0) return <div style={{ color: '#555', padding: 20, textAlign: 'center' }}>No verdicts yet. Make a move!</div>;

  return (
    <div style={{ display: 'flex', gap: 20 }}>
      <div style={{ flex: '0 0 220px', overflowY: 'auto', maxHeight: 600 }}>
        <div style={{ fontSize: 13, color: '#555', marginBottom: 10 }}>{verdicts.length} verdicts</div>
        {verdicts.map((v) => {
          const color = VERDICT_COLORS[v.verdict];
          const isSelected = selected?.verdict_id === v.verdict_id;
          return (
            <div
              key={v.verdict_id}
              onClick={() => setSelected(isSelected ? null : v)}
              style={{
                padding: '10px 12px',
                marginBottom: 6,
                borderRadius: 8,
                border: `1px solid ${isSelected ? color : '#222'}`,
                background: isSelected ? `${color}11` : '#111',
                cursor: 'pointer',
                transition: 'all 0.2s',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ color, fontFamily: 'monospace', fontWeight: 700, fontSize: 13 }}>
                  {v.verdict.toUpperCase()}
                </span>
                <span style={{ color: '#555', fontFamily: 'monospace', fontSize: 12 }}>
                  {v.q_score.total.toFixed(3)}
                </span>
              </div>
              <div style={{ fontSize: 10, color: '#444', marginTop: 4, fontFamily: 'monospace' }}>
                {v.verdict_id.slice(0, 8)}...
              </div>
            </div>
          );
        })}
      </div>
      <div style={{ flex: 1 }}>
        {selected ? <VerdictDisplay verdict={selected} /> : (
          <div style={{ color: '#444', textAlign: 'center', padding: 40 }}>Select a verdict to view details</div>
        )}
      </div>
    </div>
  );
}
