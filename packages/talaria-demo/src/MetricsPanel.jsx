import { useState, useEffect } from 'react';
import { asArray, fetchJson } from './api';

export function MetricsPanel({ lang = 'en' }) {
  const [metrics, setMetrics] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const [dogs, verdicts, history] = await Promise.all([
          fetchJson('/dogs'),
          fetchJson('/verdicts'),
          fetchJson('/state-history?limit=1').catch(() => null),
        ]);
        const latestBlock = asArray(history?.blocks)[0] ?? null;
        const recent = asArray(verdicts);
        const now = Date.now();
        const latestVerdictAge = recent[0]?.timestamp
          ? Math.max(0, Math.floor((now - new Date(recent[0].timestamp).getTime()) / 1000))
          : null;
        const latestBlockAge = latestBlock?.timestamp
          ? Math.max(0, Math.floor((now - new Date(latestBlock.timestamp).getTime()) / 1000))
          : null;
        setMetrics({
          ok: true,
          dogs: asArray(dogs),
          verdicts: recent,
          latestBlock,
          latestVerdictAge,
          latestBlockAge,
          checkedAt: new Date().toISOString(),
        });
      } catch (e) {
        console.error('Failed to load metrics', e);
        setMetrics({
          ok: false,
          error: e.message,
          checkedAt: new Date().toISOString(),
        });
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

  const verdicts = metrics?.verdicts ?? [];
  const latestBlockAge = metrics?.latestBlockAge ?? null;
  const latestVerdictAge = metrics?.latestVerdictAge ?? null;

  return (
    <div className="metrics-grid">
      <MetricCard
        label={lang === 'fr' ? 'API BACKEND' : 'BACKEND API'}
        value={metrics?.ok ? 'online' : 'offline'}
        status={metrics?.ok ? 'healthy' : 'offline'}
      />
      <MetricCard
        label={lang === 'fr' ? 'DOGS ACTIFS' : 'ACTIVE DOGS'}
        value={metrics?.ok ? metrics.dogs.length : 'n/a'}
        status={metrics?.dogs?.length > 0 ? 'active' : 'offline'}
      />
      <MetricCard
        label={lang === 'fr' ? 'VERDICTS RECENTS' : 'RECENT VERDICTS'}
        value={metrics?.ok ? verdicts.length : 'n/a'}
        status={verdicts.length > 0 ? 'active' : 'neutral'}
      />
      <MetricCard
        label={lang === 'fr' ? 'DERNIER SIGNAL' : 'LATEST SIGNAL'}
        value={latestVerdictAge != null
          ? `${latestVerdictAge}s`
          : latestBlockAge != null ? `${latestBlockAge}s` : 'n/a'}
        status={latestVerdictAge != null && latestVerdictAge < 120 ? 'active' : 'neutral'}
      />
    </div>
  );
}

function MetricCard({ label, value, status }) {
  const statusColor = status === 'healthy' ? '#4CAF50' : status === 'active' ? 'var(--gold)' : status === 'offline' ? '#d64a3a' : '#888';
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
