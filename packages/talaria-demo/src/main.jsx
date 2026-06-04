import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react'
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui'
import '@solana/wallet-adapter-react-ui/styles.css'
import './index.css'
import App from './App.jsx'

// wallet-standard: empty wallets array — standard-compatible wallets (Phantom, Backpack, Solflare…)
// are auto-detected. No per-wallet imports needed.
const ENDPOINT = 'https://api.devnet.solana.com'

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <ConnectionProvider endpoint={ENDPOINT}>
      <WalletProvider wallets={[]} autoConnect>
        <WalletModalProvider>
          <App />
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  </StrictMode>,
)
