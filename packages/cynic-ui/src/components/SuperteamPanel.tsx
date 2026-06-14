import { useEffect, useMemo, useState } from 'react';
import { ExternalLink, RefreshCw } from 'lucide-react';
import { getSuperteamOpportunities, getSuperteamSkills } from '../api';
import type { SuperteamOpportunity, SuperteamSkill } from '../types';
import { SurfacePanel } from './SurfacePanel';

type View = 'opportunities' | 'skills';

function levelTone(level: number): string {
  if (level >= 4) return 'is-success';
  if (level >= 2) return 'is-gold';
  if (level >= 1) return 'is-info';
  return 'is-muted';
}

function formatReward(opp: SuperteamOpportunity): string {
  if (opp.reward) return opp.reward;
  if (opp.reward_usd) return `$${opp.reward_usd.toLocaleString()}`;
  return 'reward unknown';
}

export function SuperteamPanel() {
  const [view, setView] = useState<View>('opportunities');
  const [opportunities, setOpportunities] = useState<SuperteamOpportunity[]>([]);
  const [skills, setSkills] = useState<SuperteamSkill[]>([]);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);

  async function refresh() {
    setLoading(true);
    setError('');
    try {
      const [nextOpportunities, nextSkills] = await Promise.all([
        getSuperteamOpportunities(),
        getSuperteamSkills(),
      ]);
      setOpportunities(nextOpportunities);
      setSkills(nextSkills);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Superteam organ unavailable');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  const stats = useMemo(() => {
    const actionable = opportunities.filter((opp) => ['new', 'reviewing'].includes(opp.status)).length;
    const strongSkills = skills.filter((skill) => skill.level >= 3 && skill.skill !== 'All').length;
    const absentSkills = skills.filter((skill) => skill.level === 0 && skill.skill !== 'All').length;
    return { actionable, strongSkills, absentSkills };
  }, [opportunities, skills]);

  const visibleOpportunities = opportunities.slice(0, 12);
  const visibleSkills = skills.filter((skill) => skill.skill !== 'All');

  return (
    <SurfacePanel
      eyebrow="SUPERTEAM"
      title="Earn intelligence"
      subtitle="Discord and Superteam Earn perception, mapped into the CYNIC operator portal."
      actions={
        <>
          <button className={`tab-btn ${view === 'opportunities' ? 'active' : ''}`} onClick={() => setView('opportunities')}>
            Opportunities
          </button>
          <button className={`tab-btn ${view === 'skills' ? 'active' : ''}`} onClick={() => setView('skills')}>
            Skills
          </button>
          <button className="button button-ghost" onClick={refresh} disabled={loading} aria-label="Refresh Superteam data">
            <RefreshCw size={14} />
            Refresh
          </button>
        </>
      }
    >
      <div className="superteam-metrics">
        <div className="metric-card">
          <span className="metric-label">Actionable</span>
          <strong>{stats.actionable}</strong>
        </div>
        <div className="metric-card">
          <span className="metric-label">L3+</span>
          <strong>{stats.strongSkills}</strong>
        </div>
        <div className="metric-card">
          <span className="metric-label">Absent</span>
          <strong>{stats.absentSkills}</strong>
        </div>
      </div>

      {error ? <div className="empty-state">{error}</div> : null}
      {loading ? <div className="loading-state">Loading Superteam organ...</div> : null}

      {!loading && !error && view === 'opportunities' ? (
        <div className="review-list">
          {visibleOpportunities.map((opp) => (
            <article className="review-card" key={opp.url}>
              <div className="review-head">
                <div>
                  <div className="review-title">{opp.title}</div>
                  <div className="review-sub">{opp.platform_excerpt || opp.platform_title || 'Platform snapshot pending.'}</div>
                </div>
                <span className="status-chip is-gold">fit {opp.fit_score}</span>
              </div>
              <div className="review-meta">
                <span>{opp.category || 'uncategorized'}</span>
                <span>{opp.capability}</span>
                <span>{formatReward(opp)}</span>
                <span>{opp.status}</span>
                <span>platform {opp.platform_status}</span>
              </div>
              <div className="review-actions">
                <a className="button button-ghost" href={opp.url} target="_blank" rel="noreferrer">
                  <ExternalLink size={14} />
                  Open Earn
                </a>
              </div>
            </article>
          ))}
        </div>
      ) : null}

      {!loading && !error && view === 'skills' ? (
        <div className="skill-matrix">
          {visibleSkills.map((skill) => (
            <article className="skill-tile" key={`${skill.category}:${skill.skill}`}>
              <div className="skill-tile-head">
                <span>{skill.skill}</span>
                <span className={`status-chip ${levelTone(skill.level)}`}>L{skill.level}</span>
              </div>
              <div className="review-sub">{skill.category} · {skill.status} · confidence {skill.confidence.toFixed(2)}</div>
              <div className="tag-row">
                {skill.evidence.slice(0, 2).map((item) => (
                  <span className="pill is-muted" key={`${skill.category}:${skill.skill}:${item.path}`}>
                    {item.kind}
                  </span>
                ))}
              </div>
            </article>
          ))}
        </div>
      ) : null}
    </SurfacePanel>
  );
}
