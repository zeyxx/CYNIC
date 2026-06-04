import { useState, useEffect } from 'react';

const API_BASE = 'https://api.talaria.build/demo';

export function ChainExplorer({ lang = 'en' }) {
  const [blocks, setBlocks] = useState([]);
  const [loading, setLoading] = useState(true);
  const [selectedBlock, setSelectedBlock] = useState(null);

  const loadData = async () => {
    try {
      const res = await fetch(`${API_BASE}/state-history?limit=20`);
      const data = await res.json();
      setBlocks(Array.isArray(data) ? data : []);
    } catch (e) {
      console.error('Failed to load state history', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  if (loading && blocks.length === 0) {
    return <div className="explorer-loading">{lang === 'fr' ? 'Accès au Proof-of-History...' : 'Accessing Proof-of-History...'}</div>;
  }

  return (
    <div className="chain-explorer-container">
      <div className="explorer-layout">
        {/* BLOCK LIST */}
        <div className="block-list-sidebar">
          <h3 className="section-label">{lang === 'fr' ? 'BLOCS D\'ÉTAT' : 'STATE BLOCKS'}</h3>
          <div className="block-items">
            {blocks.map((block) => (
              <div 
                key={block.sequence}
                className={`block-item ${selectedBlock?.sequence === block.sequence ? 'selected' : ''}`}
                onClick={() => setSelectedBlock(block)}
              >
                <div className="block-item-header">
                  <span className="block-seq">#{block.sequence}</span>
                  <span className="block-time">{new Date(block.timestamp).toLocaleTimeString()}</span>
                </div>
                <div className="block-hash-preview">{block.hash.slice(0, 16)}...</div>
              </div>
            ))}
          </div>
        </div>

        {/* BLOCK DETAIL */}
        <div className="block-detail-view">
          {selectedBlock ? (
            <div className="block-content-animate">
              <div className="block-detail-header">
                <h2 className="block-title">{lang === 'fr' ? 'BLOC' : 'BLOCK'} #{selectedBlock.sequence}</h2>
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

              <h4 className="snapshot-label">{lang === 'fr' ? 'INSTANTANÉ SYSTÈME' : 'SYSTEM SNAPSHOT'}</h4>
              <div className="snapshot-container">
                <pre className="snapshot-data">
                  {JSON.stringify(selectedBlock.data, null, 2)}
                </pre>
              </div>
            </div>
          ) : (
            <div className="explorer-empty-state">
              {lang === 'fr' ? 'Sélectionnez un bloc pour inspecter l\'état immuable' : 'Select a block to inspect its immutable state'}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
