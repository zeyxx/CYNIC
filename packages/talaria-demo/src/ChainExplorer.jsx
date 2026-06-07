import { useState, useEffect } from 'react';
import { asArray, fetchJson } from './api';

export function ChainExplorer({ lang = 'en' }) {
  const [blocks, setBlocks] = useState([]);
  const [meta, setMeta] = useState({ ok: false, error: null, checkedAt: null, chainValid: null });
  const [loading, setLoading] = useState(true);
  const [selectedBlock, setSelectedBlock] = useState(null);

  const loadData = async () => {
    try {
      const data = await fetchJson('/state-history?limit=20');
      const nextBlocks = asArray(data?.blocks);
      setBlocks(nextBlocks);
      const latestTimestamp = nextBlocks[0]?.timestamp;
      const latestAgeSecs = latestTimestamp
        ? Math.max(0, Math.floor((Date.now() - new Date(latestTimestamp).getTime()) / 1000))
        : null;
      setMeta({
        ok: true,
        error: null,
        checkedAt: new Date().toISOString(),
        chainValid: data?.chain_valid ?? null,
        blocksValid: data?.blocks_valid ?? null,
        latestAgeSecs,
      });
      setSelectedBlock(current => current ?? nextBlocks[0] ?? null);
    } catch (e) {
      console.error('Failed to load state history', e);
      setMeta({ ok: false, error: e.message, checkedAt: new Date().toISOString(), chainValid: null, blocksValid: null });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    const initial = setTimeout(loadData, 0);
    const timer = setInterval(loadData, 30000);
    return () => {
      clearTimeout(initial);
      clearInterval(timer);
    };
  }, []);

  if (loading && blocks.length === 0) {
    return <div className="explorer-loading">{lang === 'fr' ? 'Acces au Proof-of-History...' : 'Accessing Proof-of-History...'}</div>;
  }

  const latestAgeSecs = meta.latestAgeSecs ?? null;

  return (
    <div className="chain-explorer-container">
      <div className="explorer-status-line">
        <span className={`live-dot ${meta.ok ? 'on' : 'off'}`} />
        <span>{meta.ok ? (lang === 'fr' ? 'source backend active' : 'backend source active') : (lang === 'fr' ? 'source backend indisponible' : 'backend source unavailable')}</span>
        {latestAgeSecs != null && <span> · {lang === 'fr' ? 'dernier bloc' : 'latest block'} {latestAgeSecs}s</span>}
        {meta.chainValid != null && <span> · chain_valid={String(meta.chainValid)}</span>}
      </div>
      <div className="explorer-layout">
        <div className="block-list-sidebar">
          <h3 className="section-label">{lang === 'fr' ? "BLOCS D'ETAT" : 'STATE BLOCKS'}</h3>
          <div className="block-items">
            {blocks.map((block) => (
              <div
                key={block.seq ?? block.sequence ?? block.hash}
                className={`block-item ${selectedBlock?.hash === block.hash ? 'selected' : ''}`}
                onClick={() => setSelectedBlock(block)}
              >
                <div className="block-item-header">
                  <span className="block-seq">#{block.seq ?? block.sequence ?? '?'}</span>
                  <span className="block-time">{new Date(block.timestamp).toLocaleTimeString()}</span>
                </div>
                <div className="block-hash-preview">{block.hash?.slice(0, 16) ?? 'no-hash'}...</div>
              </div>
            ))}
          </div>
        </div>

        <div className="block-detail-view">
          {selectedBlock ? (
            <div className="block-content-animate">
              <div className="block-detail-header">
                <h2 className="block-title">{lang === 'fr' ? 'BLOC' : 'BLOCK'} #{selectedBlock.seq ?? selectedBlock.sequence ?? '?'}</h2>
                <span className="block-timestamp-full">{selectedBlock.timestamp}</span>
              </div>

              <div className="hash-grid">
                <div className="hash-card">
                  <div className="hash-label">HASH</div>
                  <div className="hash-value">{selectedBlock.hash}</div>
                </div>
                <div className="hash-card">
                  <div className="hash-label">PREV_HASH</div>
                  <div className="hash-value">{selectedBlock.prev_hash}</div>
                </div>
              </div>

              <h4 className="snapshot-label">{lang === 'fr' ? 'INSTANTANE SYSTEME' : 'SYSTEM SNAPSHOT'}</h4>
              <div className="snapshot-container">
                <pre className="snapshot-data">
                  {JSON.stringify(selectedBlock, null, 2)}
                </pre>
              </div>
            </div>
          ) : (
            <div className="explorer-empty-state">
              {meta.ok
                ? (lang === 'fr' ? 'Aucun bloc retourne par le backend' : 'No blocks returned by backend')
                : (meta.error ?? (lang === 'fr' ? 'Backend indisponible' : 'Backend unavailable'))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
