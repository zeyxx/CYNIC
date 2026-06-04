import { useState, useEffect } from 'react';
import { getStateHistory } from '../api';

export function BlockExplorer() {
  const [blocks, setBlocks] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    getStateHistory(20)
      .then(data => {
        setBlocks(data);
        setLoading(false);
      })
      .catch(err => {
        console.error("Failed to fetch blocks", err);
        setLoading(false);
      });
  }, []);

  if (loading) return <div>Loading timeline...</div>;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
      {blocks.map((block, i) => (
        <div key={i} style={{ 
          border: '1px solid var(--border)', 
          padding: '12px', 
          background: 'var(--surface)',
          fontFamily: 'var(--font-mono)',
          fontSize: '12px'
        }}>
          <div style={{ color: 'var(--gold)', marginBottom: '5px' }}>BLOCK #{blocks.length - i}</div>
          <pre style={{ margin: 0, overflowX: 'auto' }}>{JSON.stringify(block, null, 2)}</pre>
        </div>
      ))}
    </div>
  );
}
