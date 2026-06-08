import { useEffect, useState } from 'react';
import { getPublicObservations } from '../api';
import type { PublicObservation } from '../types';
import { AdminOraclePanel } from './AdminOraclePanel';
import { SurfacePanel } from './SurfacePanel';

interface Props {
  isAdmin: boolean;
}

export function OracleView({ isAdmin }: Props) {
  const [feed, setFeed] = useState<PublicObservation[]>([]);
  const [loading, setLoading] = useState(true);

  const loadData = async () => {
    try {
      const obs = await getPublicObservations('community', 50);
      setFeed(obs);
    } catch (e) {
      console.error('Failed to load oracle data', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    const timer = setInterval(loadData, 10000);
    return () => clearInterval(timer);
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <SurfacePanel
        eyebrow="ORACLE"
        title="Public feed"
        subtitle="Community-visible observations from the kernel. Admin review stays in a separate panel."
        actions={<span className="status-chip is-muted">{feed.length} ITEMS</span>}
      >
        {loading && feed.length === 0 ? (
          <div className="loading-state">Initializing social cortex...</div>
        ) : feed.length === 0 ? (
          <div className="empty-state">No public observatory items yet.</div>
        ) : (
          <div className="oracle-list">
            {feed.map((obs) => {
              const isOracle = obs.source_tier === 'permanent';
              const labels = [obs.tool, obs.status, obs.domain].filter(Boolean);
              const secondary = [obs.project, obs.source_tier, ...obs.tags].filter(Boolean);

              return (
                <article key={`${obs.created_at}-${obs.tool}-${obs.target}`} className={`oracle-card ${isOracle ? 'is-oracle' : ''}`}>
                  <div className="oracle-bubble-head">
                    {labels.map((label) => (
                      <span key={label} className="pill is-muted">{label}</span>
                    ))}
                    <span className="pill is-gold">{new Date(obs.created_at).toLocaleTimeString()}</span>
                  </div>
                  <div className="oracle-target">{obs.target}</div>
                  <div className="oracle-message">{obs.project}</div>
                  <div className="oracle-tags">
                    {secondary.map((item) => (
                      <span key={item} className="pill is-muted">
                        {item}
                      </span>
                    ))}
                  </div>
                </article>
              );
            })}
          </div>
        )}
      </SurfacePanel>

      {isAdmin ? <AdminOraclePanel isAdmin={isAdmin} /> : null}
    </div>
  );
}
