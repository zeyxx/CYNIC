import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '../../shared-ui/src/styles/vars.css'
import App from './App.tsx'

console.log("CYNIC-UI: Main script executed successfully");
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
