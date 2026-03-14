import { useEffect, useState } from 'react';
import { getCrystals } from '../api';
import type { Crystal } from '../types';

export function LearnedPatterns() {
  const [crystals, setCrystals] = useState<Crystal[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const data = await getCrystals();
      setCrystals(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load patterns');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  if (loading) return <div style={{ color: '#555', padding: 20, textAlign: 'center' }}>Loading patterns...</div>;
  if (error) return <div style={{ color: '#F44336', padding: 20 }}>⚠ {error}</div>;

  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))', gap: 16 }}>
      {crystals.length === 0 && (
        <div style={{ gridColumn: '1/-1', textAlign: 'center', padding: 40, color: '#444' }}>
          No crystallized patterns yet. Keep judging to form epistemic immunity.
        </div>
      )}
      {crystals.map(c => {
        const pct = Math.round(c.confidence * 100);
        const stateColor = c.state.startsWith('Crystallized') ? '#4CAF50' : '#888';
        
        return (
          <div key={c.id} style={{
            background: '#111',
            border: '1px solid #222',
            borderRadius: 12,
            padding: 16,
            display: 'flex',
            flexDirection: 'column',
            gap: 12,
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <span style={{ 
                fontSize: 10, color: stateColor, border: `1px solid ${stateColor}`, 
                padding: '2px 8px', borderRadius: 4, fontFamily: 'monospace', fontWeight: 600
              }}>
                {c.state.toUpperCase()}
              </span>
              <span style={{ fontSize: 11, color: '#444', fontFamily: 'monospace' }}>
                OBS: {c.observations}
              </span>
            </div>

            <div style={{ 
              fontSize: 14, color: '#e0e0e0', lineHeight: 1.4, 
              minHeight: 40, whiteSpace: 'pre-wrap', fontStyle: 'italic' 
            }}>
              "{c.content}"
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginTop: 'auto' }}>
              <div style={{ flex: 1, height: 4, background: '#222', borderRadius: 2 }}>
                <div style={{ 
                  height: '100%', width: `${pct}%`, 
                  background: stateColor, borderRadius: 2, 
                  boxShadow: `0 0 8px ${stateColor}44`
                }} />
              </div>
              <span style={{ fontSize: 11, color: '#666', fontFamily: 'monospace', width: 35 }}>
                {pct}%
              </span>
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 4 }}>
              <span style={{ fontSize: 10, color: '#C9A84C', letterSpacing: 1, fontWeight: 600 }}>
                {c.domain.toUpperCase()}
              </span>
              <span style={{ fontSize: 9, color: '#333' }}>
                {new Date(c.updated_at).toLocaleDateString()}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
