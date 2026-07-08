import { useQuery } from '@tanstack/react-query';
import { motion, AnimatePresence } from 'framer-motion';
import { Database, Search, Link2, AlertTriangle, Cpu } from 'lucide-react';

interface Observation {
  id: string;
  domain: string;
  kind: string;
  claim: string;
  confidence: string;
  source?: { type: string; uri: string };
  timestamp?: string;
}

export default function BuildLog() {
  const { data: observations, isLoading, isError } = useQuery<Observation[]>({
    queryKey: ['observations'],
    queryFn: async () => {
      const res = await fetch('/api/cynic/observe');
      if (!res.ok) throw new Error('Failed to fetch observations');
      return res.json();
    },
    refetchInterval: 3000, // Real-time polling
    initialData: [],
  });

  return (
    <motion.section 
      className="section"
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.6, ease: "easeOut" }}
    >
      <div className="section-heading">
        <p className="eyebrow" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Database size={16} /> Proof Ledger & Observation Surface
        </p>
        <h2>The Reality Log</h2>
      </div>
      <p className="section-copy">
        The sole source of truth for everything that is real in the organism. 
        Raw market scans, discoveries, and verified actions submitted by the CYNIC Kernel.
      </p>

      {isError && (
        <motion.div className="capture-total" style={{ borderLeft: '4px solid var(--amber)', marginTop: '1rem' }} initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <AlertTriangle color="var(--amber)" size={20} /> 
            Kernel connection degraded. Displaying cached state.
          </span>
        </motion.div>
      )}

      <div className="lifecycle-list" style={{ marginTop: '3rem' }}>
        <AnimatePresence mode="popLayout">
          {isLoading ? (
            <motion.article key="loading" className="step-card" exit={{ opacity: 0 }}>
              <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
                <Search className="animate-spin" color="var(--cyan)" size={24} />
                <h3>Syncing reality...</h3>
              </div>
            </motion.article>
          ) : observations.length > 0 ? (
            observations.map((obs, index) => (
              <motion.article 
                layout
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.95 }}
                whileHover={{ y: -4, borderColor: 'var(--green)' }}
                transition={{ duration: 0.2 }}
                className="step-card" 
                key={obs.id || index}
                style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: '24px' }}>{String(index + 1).padStart(2, '0')}</span>
                  <div style={{ background: 'rgba(20, 241, 149, 0.1)', padding: '4px 8px', borderRadius: '4px', color: 'var(--green)', fontSize: '11px', fontWeight: 'bold' }}>
                    {obs.confidence.toUpperCase()}
                  </div>
                </div>
                
                <h3 style={{ margin: 0, fontSize: '18px' }}>[{obs.domain}] {obs.kind}</h3>
                
                <p style={{ color: 'var(--text)', flexGrow: 1, margin: 0 }}>
                  <strong>Claim:</strong> {obs.claim}
                </p>

                <div style={{ 
                  marginTop: 'auto', 
                  paddingTop: '12px', 
                  borderTop: '1px solid var(--border)', 
                  display: 'flex', 
                  flexDirection: 'column', 
                  gap: '6px',
                  fontSize: '0.85em', 
                  color: 'var(--dim)' 
                }}>
                  {obs.source && (
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                      <Link2 size={14} /> {obs.source.type}: {obs.source.uri.substring(0, 30)}...
                    </div>
                  )}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                    <Cpu size={14} /> ID: {obs.id?.substring(0, 12) || 'N/A'}
                  </div>
                </div>
              </motion.article>
            ))
          ) : (
            <motion.article 
              key="empty"
              className="step-card"
              style={{ gridColumn: '1 / -1', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', textAlign: 'center', minHeight: '200px' }}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
            >
              <Database size={48} color="var(--dim)" style={{ marginBottom: '1rem' }} />
              <h3>No Observations found</h3>
              <p>Awaiting primary data from the CYNIC Kernel.</p>
            </motion.article>
          )}
        </AnimatePresence>
      </div>
    </motion.section>
  );
}
