import { useState, useEffect } from 'react';
import { useSelectedWalletAccount } from '@solana/react';
import { getAuthInput, verifyAuth } from './api';
import { ChessJudge } from './components/ChessJudge';
import { TokenScreener } from './components/TokenScreener';
import { VerdictHistory } from './components/VerdictHistory';
import { HealthIndicator } from './components/HealthIndicator';
import { KernelSettings } from './components/KernelSettings';
import { LearnedPatterns } from './components/LearnedPatterns';
import { OracleView } from './components/OracleView';
import { ChainExplorer } from './components/ChainExplorer';
import { TopologyView } from './components/TopologyView';
import './App.css';

type Tab = 'screen' | 'chess' | 'history' | 'patterns' | 'oracle' | 'explorer' | 'topology';

const TAB_LABELS: Record<Tab, string> = {
  screen: 'SCREEN',
  chess: 'CHESS',
  history: 'HISTORY',
  patterns: 'PATTERNS',
  oracle: 'ORACLE',
  explorer: 'EXPLORER',
  topology: 'TOPOLOGY',
};

function App() {
  const [tab, setTab] = useState<Tab>('screen');
  const [showSettings, setShowSettings] = useState(false);
  const [selectedAccount, setSelectedAccount, wallets] = useSelectedWalletAccount();
  const [authenticatedRole, setAuthenticatedRole] = useState<string | null>(null);
  
  const isAdmin = authenticatedRole === 'cortex' || !!(localStorage.getItem('cynic_api_key') || import.meta.env.VITE_API_KEY);

  const handleConnect = async (wallet: any) => {
    try {
      // 1. Basic connection
      await wallet.connect();
      const account = wallet.accounts[0];
      if (!account) return;

      // 2. SIWS Flow
      const { nonce, statement } = await getAuthInput();
      
      // Standard message format for SIWS
      const message = `${statement}\nNonce: ${nonce}`;
      const encodedMessage = new TextEncoder().encode(message);

      let signatureHex = '';
      
      // Try official SIWS signIn if supported by wallet, fallback to signMessage
      if (wallet.features['solana:signIn']) {
        const input = {
          domain: window.location.host,
          address: account.address,
          statement,
          nonce,
        };
        const output = await wallet.features['solana:signIn'].signIn(input);
        signatureHex = Array.from(output.signature as Uint8Array).map(b => b.toString(16).padStart(2, '0')).join('');
      } else if (wallet.features['solana:signMessage']) {
        const { signature } = await wallet.features['solana:signMessage'].signMessage({
          account,
          message: encodedMessage
        });
        signatureHex = Array.from(signature as Uint8Array).map(b => b.toString(16).padStart(2, '0')).join('');
      } else {
        throw new Error('Wallet does not support signing');
      }

      // 3. Verify on backend
      const { role } = await verifyAuth(account.address, signatureHex, nonce);
      
      setSelectedAccount(account);
      setAuthenticatedRole(role);
      console.log(`Authenticated as ${role}`);
    } catch (e) {
      console.error('SIWS failed', e);
      alert(`Authentication failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  useEffect(() => {
    if (!selectedAccount) {
      setAuthenticatedRole(null);
    }
  }, [selectedAccount]);

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', color: 'var(--text)' }}>
      <header style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 28px',
        height: 56,
        borderBottom: '1px solid var(--border)',
        background: 'var(--surface)',
        position: 'sticky',
        top: 0,
        zIndex: 50,
      }}>
        {/* Logo */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 18, flex: '0 0 auto' }}>
          <div>
            <div style={{
              fontFamily: 'var(--font-display-deco)',
              fontSize: 17,
              fontWeight: 700,
              letterSpacing: 6,
              color: 'var(--gold)',
              lineHeight: 1,
            }}>
              CYNIC
            </div>
            <div style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 8,
              color: 'var(--text-dim)',
              letterSpacing: 3,
              marginTop: 3,
              textTransform: 'uppercase',
            }}>
              Epistemic Immune System
            </div>
          </div>
          <div style={{ width: 1, height: 24, background: 'var(--border-bright)' }} />
          <HealthIndicator />
        </div>

        {/* Tabs — center */}
        <nav style={{ display: 'flex', gap: 0 }}>
          {(Object.keys(TAB_LABELS) as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              style={{
                padding: '0 18px',
                height: 56,
                border: 'none',
                borderBottom: tab === t ? '2px solid var(--gold)' : '2px solid transparent',
                background: 'transparent',
                color: tab === t ? 'var(--gold)' : 'var(--text-dim)',
                cursor: 'pointer',
                fontFamily: 'var(--font-mono)',
                fontSize: 11,
                letterSpacing: 2,
                fontWeight: tab === t ? 500 : 400,
                transition: 'color 0.15s, border-color 0.15s',
              }}
            >
              {TAB_LABELS[t]}
            </button>
          ))}
        </nav>

        {/* Right — phi constant + wallet */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 14, flex: '0 0 auto' }}>
          <span style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            color: 'var(--text-muted)',
            letterSpacing: 1,
            userSelect: 'none',
          }}>
            φ⁻¹ = 0.618034
          </span>
          
          {selectedAccount ? (
            <button 
              onClick={() => setSelectedAccount(undefined)}
              style={{
                background: 'var(--gold-glow)',
                border: '1px solid var(--gold)',
                borderRadius: 4,
                color: 'var(--gold)',
                padding: '4px 12px',
                fontSize: 11,
                fontFamily: 'var(--font-mono)',
                cursor: 'pointer'
              }}
            >
              {selectedAccount.address.slice(0, 4)}...{selectedAccount.address.slice(-4)}
            </button>
          ) : (
            <div style={{ display: 'flex', gap: 4 }}>
              {wallets.slice(0, 2).map((w: any) => (
                <button 
                  key={w.name}
                  onClick={() => handleConnect(w)}
                  title={`Connect ${w.name}`}
                  style={{
                    background: 'transparent',
                    border: '1px solid var(--border-bright)',
                    borderRadius: 4,
                    padding: '4px 8px',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center'
                  }}
                >
                  <img src={w.icon} alt={w.name} width={14} height={14} />
                </button>
              ))}
            </div>
          )}

          <button
            onClick={() => setShowSettings(true)}
            title="Kernel settings"
            style={{
              background: 'transparent',
              border: '1px solid var(--border-bright)',
              borderRadius: 6,
              color: 'var(--text-dim)',
              cursor: 'pointer',
              padding: '5px 9px',
              fontSize: 13,
              lineHeight: 1,
              transition: 'border-color 0.15s, color 0.15s',
            }}
            onMouseEnter={e => {
              (e.target as HTMLButtonElement).style.borderColor = 'var(--gold-dim)';
              (e.target as HTMLButtonElement).style.color = 'var(--gold)';
            }}
            onMouseLeave={e => {
              (e.target as HTMLButtonElement).style.borderColor = 'var(--border-bright)';
              (e.target as HTMLButtonElement).style.color = 'var(--text-dim)';
            }}
          >
            ⚙
          </button>
        </div>
      </header>

      <main style={{ padding: '28px 24px', maxWidth: 1200, margin: '0 auto' }}>
        {tab === 'screen' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="TOKEN SCREENER"
              sub="Paste a Solana address. Independent Dogs deliberate under φ-bounded doubt."
            />
            <TokenScreener />
          </div>
        )}
        {tab === 'chess' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="♟ CHESS JUDGMENT"
              sub="Each move evaluated through 6 philosophical axioms."
            />
            <ChessJudge />
          </div>
        )}
        {tab === 'history' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="VERDICT HISTORY"
              sub="Recent judgments from all Dogs."
            />
            <VerdictHistory />
          </div>
        )}
        {tab === 'patterns' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="CRYSTALLIZED PATTERNS"
              sub="Knowledge crystallized through multi-agent consensus."
            />
            <LearnedPatterns />
          </div>
        )}
        {tab === 'oracle' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="ORACLE SUPERVISION"
              sub={isAdmin ? "Real-time social interactions. Approve or edit drafts to align the organism." : "Real-time feed of the organism's social interactions and reasoning."}
            />
            <OracleView isAdmin={isAdmin} />
          </div>
        )}
        {tab === 'explorer' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="CYNIC CHAIN EXPLORER"
              sub="Immutable Proof-of-History. Browse hash-chained organism state blocks."
            />
            <ChainExplorer />
          </div>
        )}
        {tab === 'topology' && (
          <div style={{ animation: 'slide-up 0.25s ease' }}>
            <PageHeading
              title="ORGANISM TOPOLOGY"
              sub="Hardware-as-Law. Mapping the native infrastructure and the Five Dogs of Vigilance."
            />
            <TopologyView />
          </div>
        )}
      </main>

      {showSettings && <KernelSettings onClose={() => setShowSettings(false)} />}
    </div>
  );
}

function PageHeading({ title, sub }: { title: string; sub: string }) {
  return (
    <div style={{ marginBottom: 24 }}>
      <h2 style={{
        margin: 0,
        fontFamily: 'var(--font-display)',
        fontSize: 13,
        fontWeight: 600,
        color: 'var(--gold)',
        letterSpacing: 3,
        textTransform: 'uppercase',
      }}>
        {title}
      </h2>
      <p style={{
        color: 'var(--text-dim)',
        fontSize: 11,
        margin: '6px 0 0',
        fontFamily: 'var(--font-mono)',
        letterSpacing: 0.5,
      }}>
        {sub}
      </p>
    </div>
  );
}

export default App;
