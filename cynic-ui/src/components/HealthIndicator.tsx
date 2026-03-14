import { useEffect, useState } from 'react';
import { checkHealth } from '../api';


export function HealthIndicator() {
  const [status, setStatus] = useState<'checking' | 'sovereign' | 'offline'>('checking');
  const [version, setVersion] = useState('');

  useEffect(() => {
    const check = async () => {
      try {
        const h = await checkHealth();
        setStatus(h.status === 'sovereign' ? 'sovereign' : 'offline');
        setVersion(h.version || '');
      } catch {
        setStatus('offline');
      }
    };
    check();
    const interval = setInterval(check, 30000);
    return () => clearInterval(interval);
  }, []);

  const color = status === 'sovereign' ? '#4CAF50' : status === 'offline' ? '#F44336' : '#FF9800';
  const label = status === 'sovereign' ? 'SOVEREIGN' : status === 'offline' ? 'OFFLINE' : 'CHECKING';

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <div style={{
        width: 10,
        height: 10,
        borderRadius: '50%',
        background: color,
        boxShadow: `0 0 6px ${color}`,
        animation: status === 'checking' ? 'pulse 1s infinite' : 'none',
      }} />
      <span style={{ fontSize: 12, color, fontFamily: 'monospace', letterSpacing: 1 }}>
        {label}{version ? ` v${version}` : ''}
      </span>
    </div>
  );
}
