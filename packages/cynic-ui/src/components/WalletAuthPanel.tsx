import { useEffect, useMemo, useState } from 'react';
import { useConnect, useDisconnect, useWallets } from '@wallet-standard/react';
import type { UiWallet, UiWalletAccount } from '@wallet-standard/ui-core';
import { useSelectedWalletAccount, useSignMessage } from '@solana/react';
import { buildAuthMessage, clearAuthSession, loadAuthSession, saveAuthSession } from '../auth';
import { getAuthInput, verifyAuth } from '../api';
import { SurfacePanel } from './SurfacePanel';

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function WalletRow({
  wallet,
  selectedAddress,
  onSelectAccount,
}: {
  wallet: UiWallet;
  selectedAddress: string | null;
  onSelectAccount: (address: string) => void;
}) {
  const [connecting, connect] = useConnect(wallet);
  const [disconnecting, disconnect] = useDisconnect(wallet);
  const connected = wallet.accounts.length > 0;
  const isSelected = wallet.accounts.some((account: UiWalletAccount) => account.address === selectedAddress);

  const handleConnect = async () => {
    const accounts = await connect();
    const first = accounts[0];
    if (first) {
      onSelectAccount(first.address);
    }
  };

  return (
    <div className={`wallet-row ${isSelected ? 'is-selected' : ''}`}>
      <div className="wallet-row-head">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          <div className="wallet-name">{wallet.name}</div>
          <div className="wallet-sub">{wallet.accounts.length > 0 ? `${wallet.accounts.length} account(s)` : 'not connected'}</div>
        </div>
        <div className="wallet-meta">
          {isSelected ? <span className="pill is-gold">SELECTED</span> : null}
        </div>
      </div>
      <div className="wallet-actions">
        {connected ? (
          <button onClick={() => disconnect()} disabled={disconnecting} className="button button-ghost">
            {disconnecting ? 'DISCONNECTING' : 'DISCONNECT'}
          </button>
        ) : (
          <button onClick={handleConnect} disabled={connecting} className="button button-primary">
            {connecting ? 'CONNECTING' : 'CONNECT'}
          </button>
        )}
      </div>
    </div>
  );
}

function SignedInControls({
  selectedWalletAccount,
  onSignedIn,
  onSignedOut,
}: {
  selectedWalletAccount: NonNullable<ReturnType<typeof useSelectedWalletAccount>[0]>;
  onSignedIn: () => void;
  onSignedOut: () => void;
}) {
  const signMessage = useSignMessage(selectedWalletAccount);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  const handleSignIn = async () => {
    setBusy(true);
    setStatus(null);
    try {
      const auth = await getAuthInput();
      const message = buildAuthMessage(auth);
      const encoded = new TextEncoder().encode(message);
      const { signature } = await signMessage({ message: encoded });
      const verified = await verifyAuth(
        selectedWalletAccount.address,
        toHex(signature),
        auth.nonce,
        auth.timestamp,
      );
      const tok = verified.session_token;
      saveAuthSession({
        session_token: tok,
        role: verified.role,
        expires_at: verified.expires_at,
        address: verified.address,
      });
      setStatus(`Signed in as ${selectedWalletAccount.address}`);
      onSignedIn();
    } catch (error) {
      clearAuthSession();
      setStatus(error instanceof Error ? error.message : 'Wallet sign-in failed');
    } finally {
      setBusy(false);
    }
  };

  const handleSignOut = () => {
    clearAuthSession();
    setStatus('Session cleared');
    onSignedOut();
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10, alignItems: 'flex-end' }}>
      <div className="review-actions">
        <button onClick={handleSignOut} className="button button-ghost">
          SIGN OUT
        </button>
        <button onClick={handleSignIn} disabled={busy} className="button button-primary">
          {busy ? 'SIGNING' : 'SIGN IN'}
        </button>
      </div>
      {status ? <div className="helper-text">{status}</div> : null}
    </div>
  );
}

export function WalletAuthPanel() {
  const wallets = useWallets();
  const [selectedWalletAccount, setSelectedWalletAccount] = useSelectedWalletAccount();
  const [session, setSession] = useState(loadAuthSession());
  const selectedWallet = useMemo(() => {
    if (!selectedWalletAccount) return null;
    for (const wallet of wallets) {
      if (wallet.accounts.some((account: UiWalletAccount) => account.address === selectedWalletAccount.address)) {
        return wallet;
      }
    }
    return null;
  }, [selectedWalletAccount, wallets]);

  useEffect(() => {
    setSession(loadAuthSession());
  }, []);

  const handleSelectAccount = (address: string) => {
    const account = wallets
      .flatMap((wallet) => wallet.accounts)
      .find((candidate) => candidate.address === address);
    if (account) {
      setSelectedWalletAccount(account);
    }
  };

  return (
    <SurfacePanel
      eyebrow="OPERATOR AUTH"
      title="Wallet session"
      subtitle="Protected endpoints accept a short-lived wallet session. The browser never stores an API key."
      actions={
        <>
          {session ? <span className="status-chip is-success">SESSION ACTIVE</span> : <span className="status-chip is-muted">SESSION REQUIRED</span>}
          {selectedWalletAccount ? <span className="status-chip is-gold">{selectedWalletAccount.address.slice(0, 8)}…</span> : null}
        </>
      }
    >
      <div className="wallet-grid">
        <div className="surface-card" style={{ minHeight: 0 }}>
          <div className="meta-line">
            <span className="pill is-muted">SESSION</span>
            <span className="pill is-muted">{session ? session.role : 'not signed in'}</span>
          </div>
          {selectedWallet ? (
            <div className="wallet-sub">
              Selected wallet: <span style={{ color: 'var(--text)' }}>{selectedWallet.name}</span>
              {selectedWalletAccount ? <> · {selectedWalletAccount.address}</> : null}
            </div>
          ) : (
            <div className="wallet-sub">No wallet selected yet.</div>
          )}
          {session ? (
            <div className="helper-text">Expires at {new Date(session.expires_at * 1000).toLocaleString()}</div>
          ) : null}
        </div>

        <div className="surface-card" style={{ minHeight: 0 }}>
          <div className="wallet-row-head" style={{ marginBottom: 10 }}>
            <div className="wallet-name">Session actions</div>
            {selectedWalletAccount ? <span className="pill is-gold">READY</span> : <span className="pill is-muted">SELECT A WALLET</span>}
          </div>
          {selectedWalletAccount ? (
            <SignedInControls
              selectedWalletAccount={selectedWalletAccount}
              onSignedIn={() => setSession(loadAuthSession())}
              onSignedOut={() => setSession(null)}
            />
          ) : (
            <div className="helper-text">Connect a wallet, then sign the challenge to unlock protected endpoints.</div>
          )}
        </div>
      </div>

      <div className="surface-subtle-line" />

      <div className="wallet-grid">
        {wallets.length === 0 ? (
          <div className="empty-state">No Wallet Standard provider detected in this browser.</div>
        ) : (
          wallets.map((wallet) => (
            <WalletRow
              key={`${wallet.name}:${wallet.accounts[0]?.address ?? 'no-account'}`}
              wallet={wallet}
              selectedAddress={selectedWalletAccount?.address ?? null}
              onSelectAccount={handleSelectAccount}
            />
          ))
        )}
      </div>
    </SurfacePanel>
  );
}
