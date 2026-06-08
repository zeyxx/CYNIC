import { useEffect, useState } from 'react';
import { getStateHistory } from '../api';
import type { StateHistoryResponse, StateHistoryBlock } from '../types';
import { SurfacePanel } from './SurfacePanel';

function shortHash(value: string): string {
  return value.length > 14 ? `${value.slice(0, 10)}…${value.slice(-4)}` : value;
}

function BlockCard({ block }: { block: StateHistoryBlock }) {
  const blockBadges = [
    `${block.system.healthy_dogs}/${block.system.total_dogs} dogs`,
    `${block.system.crystals_forming} forming`,
    `${block.system.crystals_crystallized} crystallized`,
    `${block.system.verdict_count} verdicts`,
  ];

  return (
    <article className="timeline-item">
      <div className="timeline-head">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <div className="timeline-meta">
            <span className="pill is-gold">SEQ {block.seq}</span>
            <span>{new Date(block.timestamp).toLocaleString()}</span>
            <span className="pill is-muted">{block.system.status}</span>
          </div>
          <div className="timeline-title">State block {block.seq}</div>
          <div className="timeline-sub">{block.system.healthy_dogs} healthy dogs across {block.system.total_dogs} total.</div>
        </div>
        <div className="timeline-badges">
          {blockBadges.map((badge) => (
            <span key={badge} className="pill is-muted">{badge}</span>
          ))}
        </div>
      </div>

      <div className="meta-line">
        <span>hash {shortHash(block.hash)}</span>
        <span>prev {shortHash(block.prev_hash)}</span>
        <span>cpu {block.resource.cpu_pct.toFixed(1)}%</span>
        <span>mem {block.resource.memory_used_gb.toFixed(2)} GB</span>
        <span>disk {block.resource.disk_avail_gb.toFixed(2)} GB</span>
        <span>uptime {Math.floor(block.resource.uptime_secs / 60)} min</span>
      </div>

      {block.dogs.length > 0 ? (
        <div className="pill-row">
          {block.dogs.slice(0, 6).map((dog) => (
            <span key={dog.id} className="pill is-muted">
              {dog.id} · {dog.circuit} · {(dog.success_rate * 100).toFixed(1)}%
            </span>
          ))}
        </div>
      ) : null}

      <details>
        <summary>RAW SNAPSHOT</summary>
        <pre className="json-block">{JSON.stringify(block, null, 2)}</pre>
      </details>
    </article>
  );
}

export function BlockExplorer() {
  const [history, setHistory] = useState<StateHistoryResponse>({ blocks: [], count: 0, chain_valid: false, blocks_valid: false });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    getStateHistory(20)
      .then((data) => {
        setHistory(data);
        setLoading(false);
      })
      .catch((err) => {
        console.error('Failed to fetch blocks', err);
        setLoading(false);
      });
  }, []);

  return (
    <SurfacePanel
      eyebrow="TIMELINE"
      title="State chain"
      subtitle="Hash-chained organism snapshots. The chain is tamper-evident; the panel stays readable."
      actions={
        <>
          <span className={`status-chip ${history.chain_valid ? 'is-success' : 'is-danger'}`}>
            CHAIN {history.chain_valid ? 'VALID' : 'BROKEN'}
          </span>
          <span className={`status-chip ${history.blocks_valid ? 'is-success' : 'is-danger'}`}>
            BLOCKS {history.blocks_valid ? 'VALID' : 'BROKEN'}
          </span>
          <span className="status-chip is-muted">{history.count} BLOCKS</span>
        </>
      }
    >
      {loading ? (
        <div className="loading-state">Loading timeline...</div>
      ) : history.blocks.length === 0 ? (
        <div className="empty-state">No state blocks available yet.</div>
      ) : (
        <div className="timeline-list">
          {history.blocks.map((block) => (
            <BlockCard key={block.hash} block={block} />
          ))}
        </div>
      )}
    </SurfacePanel>
  );
}
