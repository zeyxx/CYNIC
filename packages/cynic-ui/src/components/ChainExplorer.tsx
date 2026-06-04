import { useState, useEffect } from 'react';
import { getStateHistory } from '../api';

export function ChainExplorer() {
  const [blocks, setBlocks] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedBlock, setSelectedBlock] = useState<any | null>(null);

  const loadData = async () => {
    try {
      const data = await getStateHistory(50);
      setBlocks(data);
    } catch (e) {
      console.error('Failed to load state history', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  if (loading && blocks.length === 0) {
    return <div style={{ color: 'var(--text-dim)', padding: 20, fontFamily: 'var(--font-mono)', fontSize: 12 }}>Accessing Proof-of-History...</div>;
  }

  return (
    <div style={{ display: 'flex', gap: 24, height: 'calc(100vh - 160px)' }}>
      {/* ── BLOCK LIST ── */}
      <div style={{ flex: '0 0 320px', overflowY: 'auto', borderRight: '1px solid var(--border)', paddingRight: 20 }}>
        <h3 style={{ margin: '0 0 16px 0', fontSize: 13, color: 'var(--text-dim)', letterSpacing: 2 }}>
          STATE BLOCKS
        </h3>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {blocks.map((block) => {
            const isSelected = selectedBlock?.sequence === block.sequence;
            return (
              <div 
                key={block.sequence}
                onClick={() => setSelectedBlock(block)}
                style={{
                  padding: '12px',
                  background: isSelected ? 'var(--gold-glow)' : 'var(--card)',
                  border: isSelected ? '1px solid var(--gold)' : '1px solid var(--border)',
                  borderRadius: 8,
                  cursor: 'pointer',
                  transition: 'all 0.2s'
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                  <span style={{ color: isSelected ? 'var(--gold)' : 'var(--text)', fontSize: 12, fontWeight: 700 }}>
                    #{block.sequence}
                  </span>
                  <span style={{ color: 'var(--text-muted)', fontSize: 10 }}>
                    {new Date(block.timestamp).toLocaleTimeString()}
                  </span>
                </div>
                <div style={{ fontSize: 10, color: 'var(--text-dim)', fontFamily: 'var(--font-mono)', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {block.hash.slice(0, 16)}...
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* ── BLOCK DETAIL ── */}
      <div style={{ flex: 1, overflowY: 'auto', paddingLeft: 4 }}>
        {selectedBlock ? (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 20 }}>
              <h2 style={{ margin: 0, fontSize: 18, color: 'var(--gold)' }}>BLOCK #{selectedBlock.sequence}</h2>
              <span style={{ fontSize: 11, color: 'var(--text-muted)', background: 'var(--surface)', padding: '4px 8px', borderRadius: 4, border: '1px solid var(--border)' }}>
                {selectedBlock.timestamp}
              </span>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16, marginBottom: 24 }}>
              <InfoCard label="HASH" value={selectedBlock.hash} />
              <InfoCard label="PREV_HASH" value={selectedBlock.prev_hash} />
            </div>

            <h4 style={{ fontSize: 11, color: 'var(--text-muted)', letterSpacing: 2, marginBottom: 12 }}>SYSTEM SNAPSHOT</h4>
            <div style={{ background: '#0a0a0a', border: '1px solid var(--border)', borderRadius: 8, padding: 16, overflowX: 'auto' }}>
              <pre style={{ margin: 0, fontSize: 11, color: '#888', lineHeight: 1.5 }}>
                {JSON.stringify(selectedBlock.data, null, 2)}
              </pre>
            </div>
          </div>
        ) : (
          <div style={{ height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', fontSize: 12 }}>
            Select a block to inspect its immutable state
          </div>
        )}
      </div>
    </div>
  );
}

function InfoCard({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ background: 'var(--surface)', border: '1px solid var(--border)', borderRadius: 8, padding: '12px' }}>
      <div style={{ fontSize: 9, color: 'var(--text-muted)', letterSpacing: 1.5, marginBottom: 6 }}>{label}</div>
      <div style={{ fontSize: 11, color: 'var(--text-dim)', fontFamily: 'var(--font-mono)', wordBreak: 'break-all' }}>{value}</div>
    </div>
  );
}
