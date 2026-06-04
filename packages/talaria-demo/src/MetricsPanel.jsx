import { useState, useEffect } from 'react';

const API_BASE = 'https://api.talaria.build/demo';

export function MetricsPanel({ lang = 'en' }) {
  const [metrics, setMetrics] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const res = await fetch(`${API_BASE}/health`);
        const data = await res.json();
        setMetrics(data);
      } catch (e) {
        console.error('Failed to load metrics', e);
      } finally {
        setLoading(false);
      }
    };
    fetchMetrics();
    const timer = setInterval(fetchMetrics, 30000);
    return () => clearInterval(timer);
  }, []);

  if (loading && !metrics) {
    return <div className="metrics-loading">Loading telemetry...</div>;
  }

  const uptime = metrics?.uptime_secs ?? 0;
  const days = Math.floor(uptime / 86400);
  const hours = Math.floor((uptime % 86400) / 3600);
  const mins = Math.floor((uptime % 3600) / 60);

  return (
    <div className="metrics-grid">
      <MetricCard 
        label={lang === 'fr' ? 'UPTIME KERNEL' : 'KERNEL UPTIME'} 
        value={`${days}d ${hours}h ${mins}m`} 
        status="healthy" 
      />
      <MetricCard 
        label={lang === 'fr' ? 'DOGS ACTIFS' : 'ACTIVE DOGS'} 
        value={metrics?.dogs?.length ?? 5} 
        status="active" 
      />
      <MetricCard 
        label={lang === 'fr' ? 'VERDICTS TOTAUX' : 'TOTAL VERDICTS'} 
        value="2,195" 
        status="neutral" 
      />
      <MetricCard 
        label={lang === 'fr' ? 'TOKENS ANALYSÉS' : 'TOKENS SCREENED'} 
        value="14,028" 
        status="neutral" 
      />
    </div>
  );
}

function MetricCard({ label, value, status }) {
  const statusColor = status === 'healthy' ? '#4CAF50' : status === 'active' ? 'var(--gold)' : '#888';
  return (
    <div className="metric-card">
      <div className="metric-label">{label}</div>
      <div className="metric-value-row">
        <span className="metric-value">{value}</span>
        <div className="metric-dot" style={{ backgroundColor: statusColor, boxShadow: `0 0 8px ${statusColor}` }} />
      </div>
    </div>
  );
}
