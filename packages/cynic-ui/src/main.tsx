import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { SelectedWalletAccountContextProvider } from '@solana/react';
import '../../shared-ui/src/styles/vars.css'
import App from './App.tsx'

const SELECTED_WALLET_ACCOUNT_LS_KEY = 'cynic_selected_wallet_account';

function getSelectedWalletAccount() {
  return window.localStorage.getItem(SELECTED_WALLET_ACCOUNT_LS_KEY);
}

function storeSelectedWalletAccount(accountKey: string) {
  window.localStorage.setItem(SELECTED_WALLET_ACCOUNT_LS_KEY, accountKey);
}

function deleteSelectedWalletAccount() {
  window.localStorage.removeItem(SELECTED_WALLET_ACCOUNT_LS_KEY);
}

console.log("CYNIC-UI: Main script executed successfully");
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <SelectedWalletAccountContextProvider
      filterWallets={() => true}
      stateSync={{
        getSelectedWallet: getSelectedWalletAccount,
        storeSelectedWallet: storeSelectedWalletAccount,
        deleteSelectedWallet: deleteSelectedWalletAccount,
      }}
    >
      <App />
    </SelectedWalletAccountContextProvider>
  </StrictMode>,
)
