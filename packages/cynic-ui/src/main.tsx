import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { SelectedWalletAccountContextProvider } from '@solana/react'

import './index.css'
import App from './App.tsx'

const NO_FILTER = () => true;
const NO_SYNC = {
  deleteSelectedWallet: () => {},
  getSelectedWallet: () => null,
  storeSelectedWallet: () => {},
};

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <SelectedWalletAccountContextProvider filterWallets={NO_FILTER} stateSync={NO_SYNC}>
      <App />
    </SelectedWalletAccountContextProvider>
  </StrictMode>,
)
