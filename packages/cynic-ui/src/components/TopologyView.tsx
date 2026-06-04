import { useState, useEffect } from 'react';

const DOGS_DATA = [
  {
    id: 'deterministic-dog',
    name: 'The Cerberus',
    role: 'Structural Integrity',
    tradition: 'Formal Logic & Code',
    description: 'Checks for hard rule violations, schema mismatches, and mathematical impossibility. He does not reason; he calculates the boundaries of the possible.',
    axiom: 'PHI & VERIFY'
  },
  {
    id: 'cynic-dog',
    name: 'Diogenes',
    role: 'Axiomatic Skepticism',
    tradition: 'Cynicism (Antisthenes)',
    description: 'Looks for the "honest man" in the proposal. Bites performative theatre and hidden incentives. He demands to know why this matters for the sovereign.',
    axiom: 'FIDELITY & SOVEREIGNTY'
  },
  {
    id: 'hermetic-dog',
    name: 'Trismegistus',
    role: 'Pattern Recognition',
    tradition: 'Hermeticism / Kybalion',
    description: 'Analyzes correspondence: "as above, so below". Does the token behavior match the social claim? Detects polarity and gender (market dynamics).',
    axiom: 'PHI & CULTURE'
  },
  {
    id: 'stoic-dog',
    name: 'Epictetus',
    role: 'Resource Utility',
    tradition: 'Stoicism',
    description: 'Judges what is within control and what is waste. Focuses on the efficiency of capital and energy. He hates bloat and loves the "Burn".',
    axiom: 'BURN'
  },
  {
    id: 'zen-dog',
    name: 'The Archer',
    role: 'Direct Action',
    tradition: 'Eastern (Zen/Dao)',
    description: 'Looks for Wu-Wei (effortless action). Does the proposal flow with the market or fight it? Detects authenticity through the silence between the words.',
    axiom: 'FIDELITY & KENOSIS'
  }
];

export function TopologyView() {
  const [telemetry, setTelemetry] = useState<any>(null);

  useEffect(() => {
    // In a real scenario, this would fetch from /health or a new /topology endpoint
    const mockTelemetry = {
      kernel: { cpu: 12, mem: 450, status: 'sovereign', uptime: '14d 2h' },
      dogs: { active: 5, load: 'optimal', inference: 'local (RTX 4090)' },
      energy: { source: 'native', efficiency: '0.618', cost_per_verdict: '0.0042 kWh' },
      chain: { blocks: 4102, tps: 0.8, latency: '42ms' }
    };
    setTelemetry(mockTelemetry);
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 32 }}>
      {/* ── THE DOGS (Culture Section) ── */}
      <div>
        <h3 style={{ color: 'var(--gold)', letterSpacing: 2, fontSize: 14, marginBottom: 20 }}>THE FIVE DOGS (VIGILANCE TIER)</h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: 16 }}>
          {DOGS_DATA.map(dog => (
            <div key={dog.id} style={{ background: 'var(--card)', border: '1px solid var(--border)', borderRadius: 12, padding: 20 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 12 }}>
                <div>
                  <div style={{ color: 'var(--gold)', fontSize: 16, fontFamily: 'var(--font-display)', fontWeight: 600 }}>{dog.name}</div>
                  <div style={{ color: 'var(--text-muted)', fontSize: 10, fontFamily: 'var(--font-mono)', letterSpacing: 1 }}>{dog.id.toUpperCase()}</div>
                </div>
                <div style={{ background: 'var(--gold-glow)', color: 'var(--gold)', padding: '4px 8px', borderRadius: 4, fontSize: 10, fontWeight: 700 }}>{dog.axiom}</div>
              </div>
              <div style={{ color: 'var(--text-dim)', fontSize: 11, fontStyle: 'italic', marginBottom: 10 }}>{dog.tradition} — {dog.role}</div>
              <div style={{ color: 'var(--text)', fontSize: 13, lineHeight: 1.5 }}>{dog.description}</div>
            </div>
          ))}
        </div>
      </div>

      {/* ── MACHINE TOPOLOGY (Hardware-as-Law) ── */}
      <div>
        <h3 style={{ color: 'var(--gold)', letterSpacing: 2, fontSize: 14, marginBottom: 20 }}>NATIVE INFRASTRUCTURE (TOPOLOGY)</h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: 16 }}>
          <TechCard title="KERNEL (CORTEX)" data={telemetry?.kernel} icon="🧠" />
          <TechCard title="DOGS (INFERENCE)" data={telemetry?.dogs} icon="🐕" />
          <TechCard title="ENERGY (NATIVE)" data={telemetry?.energy} icon="⚡" />
          <TechCard title="POH CHAIN" data={telemetry?.chain} icon="⛓️" />
        </div>
      </div>

      {/* ── CULTURE NOTE ── */}
      <div style={{ background: 'var(--gold-glow)', border: '1px dashed var(--gold)', borderRadius: 8, padding: 20, textAlign: 'center' }}>
        <div style={{ color: 'var(--gold)', fontFamily: 'var(--font-display)', fontSize: 18, marginBottom: 8 }}>"Sovereign ground, bounded doubt."</div>
        <p style={{ color: 'var(--text-dim)', fontSize: 13, maxWidth: 600, margin: '0 auto', lineHeight: 1.6 }}>
          We chose the name <b>Dogs</b> because unlike "Agents", a Dog is faithful to its owner, not its programmer. 
          They bark when they smell a lie. They don't seek consensus to be polite; they seek the Φ-point of truth. 
          The infrastructure is native because virtualized trust is no trust at all.
        </p>
      </div>
    </div>
  );
}

function TechCard({ title, data, icon }: { title: string, data: any, icon: string }) {
  return (
    <div style={{ background: 'var(--surface)', border: '1px solid var(--border-bright)', borderRadius: 8, padding: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
        <span style={{ fontSize: 20 }}>{icon}</span>
        <span style={{ fontSize: 12, color: 'var(--gold)', letterSpacing: 1.5, fontWeight: 600 }}>{title}</span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        {data && Object.entries(data).map(([k, v]: [string, any]) => (
          <div key={k} style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11 }}>
            <span style={{ color: 'var(--text-muted)', textTransform: 'uppercase' }}>{k}</span>
            <span style={{ color: 'var(--text)', fontFamily: 'var(--font-mono)' }}>{v}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
