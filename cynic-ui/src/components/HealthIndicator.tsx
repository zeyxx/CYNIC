import { useEffect, useState } from 'react';
import { checkHealth } from '../api';


export function HealthIndicator() {
  const [status, setStatus] = useState<'checking' | 'online' | 'offline'>('checking');
  const [kernelStatus, setKernelStatus] = useState('');
  const [version, setVersion] = useState('');

  useEffect(() => {
    let cancelled = false;
    const check = async () => {
      try {
        const h = await checkHealth();
        if (!cancelled) {
          setStatus('online');
          setKernelStatus(h.status || '');
          setVersion(h.version || '');
        }
      } catch {
        if (!cancelled) setStatus('offline');
      }
    };
    check();
    const interval = setInterval(check, 30000);
    return () => { cancelled = true; clearInterval(interval); };
  }, []);

  const color = status === 'online' ? '#4CAF50' : status === 'offline' ? '#F44336' : '#FF9800';
  const label = status === 'online' ? (kernelStatus.toUpperCase() || 'ONLINE') : status === 'offline' ? 'OFFLINE' : 'CHECKING';

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
