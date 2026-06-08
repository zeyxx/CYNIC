import { useEffect, useState } from 'react';
import { getStateHistory } from '../api';
import type { StateHistoryBlock } from '../types';
import { SurfacePanel } from './SurfacePanel';

export function ChainExplorer() {
  const [blocks, setBlocks] = useState<StateHistoryBlock[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedBlock, setSelectedBlock] = useState<StateHistoryBlock | null>(null);

  useEffect(() => {
    getStateHistory(50)
      .then((data) => {
        setBlocks(data.blocks);
        setSelectedBlock((current) => current ?? data.blocks[0] ?? null);
      })
      .catch((e) => {
        console.error('Failed to load state history', e);
      })
      .finally(() => {
        setLoading(false);
      });
  }, []);

  if (loading && blocks.length === 0) {
    return <div className="loading-state">Accessing Proof-of-History...</div>;
  }

  return (
    <SurfacePanel
      eyebrow="CHAIN"
      title="State blocks"
      subtitle="Compact explorer for the organism state log."
      actions={<span className="status-chip is-muted">{blocks.length} BLOCKS</span>}
    >
      <div style={{ display: 'grid', gridTemplateColumns: '320px 1fr', gap: 18, alignItems: 'start' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, maxHeight: '70vh', overflow: 'auto' }}>
          {blocks.map((block) => {
            const isSelected = selectedBlock?.hash === block.hash;
            return (
              <button
                key={block.hash}
                onClick={() => setSelectedBlock(block)}
                className={`wallet-row ${isSelected ? 'is-selected' : ''}`}
                style={{ textAlign: 'left' }}
              >
                <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                  <div className="wallet-name">Block {block.seq}</div>
                  <div className="wallet-sub">{new Date(block.timestamp).toLocaleString()}</div>
                </div>
                <div className="wallet-meta">
                  <span className="pill is-muted">{block.system.status}</span>
                </div>
              </button>
            );
          })}
        </div>

        {selectedBlock ? (
          <div className="surface-card" style={{ minHeight: 0 }}>
            <div className="meta-line">
              <span className="pill is-gold">SEQ {selectedBlock.seq}</span>
              <span>{selectedBlock.timestamp}</span>
            </div>
            <div className="wallet-sub">hash {selectedBlock.hash}</div>
            <div className="wallet-sub">prev {selectedBlock.prev_hash}</div>
            <div className="pill-row">
              <span className="pill is-muted">dogs {selectedBlock.system.healthy_dogs}/{selectedBlock.system.total_dogs}</span>
              <span className="pill is-muted">verdicts {selectedBlock.system.verdict_count}</span>
              <span className="pill is-muted">tokens {selectedBlock.system.total_tokens}</span>
            </div>
            <pre className="json-block">{JSON.stringify(selectedBlock, null, 2)}</pre>
          </div>
        ) : (
          <div className="empty-state">Select a block to inspect its immutable state.</div>
        )}
      </div>
    </SurfacePanel>
  );
}
