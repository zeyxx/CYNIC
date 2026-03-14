import { useEffect, useState } from 'react';
import { checkHealth } from '../api';

export function HealthDot() {
  const [status, setStatus] = useState<'unknown' | 'up' | 'down'>('unknown');

  const poll = async () => {
    try {
      const h = await checkHealth();
      setStatus(h.status === 'sovereign' ? 'up' : 'down');
    } catch {
      setStatus('down');
    }
  };

  useEffect(() => {
    poll();
    const id = setInterval(poll, 10000);
    return () => clearInterval(id);
  }, []);

  const color = status === 'up' ? '#4caf50' : status === 'down' ? '#f44336' : '#666';
  const label = status === 'up' ? 'sovereign' : status === 'down' ? 'offline' : '…';

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: '#888' }}>
      <span style={{
        width: '8px', height: '8px', borderRadius: '50%',
        background: color,
        boxShadow: status === 'up' ? `0 0 6px ${color}` : 'none',
        display: 'inline-block',
        transition: 'all 0.3s',
      }} />
      <span>{label}</span>
    </div>
  );
}
