"use client";

import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { Server, Activity, Terminal, Code2, RefreshCw, KeyRound, LogOut } from "lucide-react";
import { useEffect, useState, useCallback } from "react";
import bs58 from "bs58";

export default function Home() {
  const { publicKey, connected, signMessage, disconnect } = useWallet();
  const [mounted, setMounted] = useState(false);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isSigning, setIsSigning] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  // Handle wallet disconnect
  useEffect(() => {
    if (!connected) setIsAuthenticated(false);
  }, [connected]);

  const handleSign = useCallback(async () => {
    try {
      if (!publicKey || !signMessage) return;
      setIsSigning(true);
      const message = new TextEncoder().encode("Authenticate to CYNIC Cloud\n\nProve ownership to access bare-metal infrastructure.\nNonce: " + Date.now());
      const signature = await signMessage(message);
      if (signature) {
        setIsAuthenticated(true);
      }
    } catch (err) {
      console.error("Signature failed", err);
    } finally {
      setIsSigning(false);
    }
  }, [publicKey, signMessage]);

  const handleAction = useCallback(async (actionType: string) => {
    try {
      if (!publicKey || !signMessage) return;
      setIsSigning(true);
      
      const messageStr = `Authorize deployment on CYNIC Cloud\nAction: ${actionType}\nNonce: ${Date.now()}`;
      const message = new TextEncoder().encode(messageStr);
      
      const signature = await signMessage(message);
      
      if (signature) {
        const payload = {
          public_key: publicKey.toBase58(),
          message: messageStr,
          signature: bs58.encode(signature),
          action: actionType
        };
        
        // Use local API for testing, or production if needed
        const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:4000/api/deploy";
        const response = await fetch(apiUrl, {
          method: "POST",
          headers: {
            "Content-Type": "application/json"
          },
          body: JSON.stringify(payload)
        });
        
        const data = await response.json();
        if (response.ok) {
          alert(`Action success!\n${data.logs || 'Done'}`);
        } else {
          alert(`Action failed!\n${data}`);
        }
      }
    } catch (err) {
      console.error("Action failed", err);
      alert("Signature or network failed");
    } finally {
      setIsSigning(false);
    }
  }, [publicKey, signMessage]);

  if (!mounted) return null;

  return (
    <main className="flex-1 flex flex-col items-center justify-center p-6 bg-gradient-to-b from-neutral-950 to-neutral-900 min-h-screen">
      <div className="max-w-3xl w-full">
        {/* Header */}
        <div className="flex justify-between items-center mb-12">
          <div className="flex items-center gap-3">
            <div className="h-10 w-10 rounded-xl bg-gradient-to-br from-purple-500 to-blue-600 flex items-center justify-center shadow-lg shadow-purple-500/20">
              <Server className="text-white h-5 w-5" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-white">CYNIC Cloud</h1>
          </div>
          <div className="flex items-center gap-4">
            {isAuthenticated && (
              <button 
                onClick={() => { setIsAuthenticated(false); disconnect(); }}
                className="text-neutral-400 hover:text-white transition-colors flex items-center gap-2 text-sm"
              >
                <LogOut className="w-4 h-4" /> Disconnect
              </button>
            )}
            {!isAuthenticated && <WalletMultiButton className="!bg-neutral-800 hover:!bg-neutral-700 transition-colors !rounded-lg !h-10" />}
          </div>
        </div>

        {/* Not connected state */}
        {!connected && (
          <div className="flex flex-col items-center justify-center py-24 text-center border border-neutral-800 rounded-2xl bg-neutral-900/50 backdrop-blur-sm">
            <Activity className="h-12 w-12 text-neutral-500 mb-6" />
            <h2 className="text-2xl font-semibold mb-3 text-neutral-200">Connect to access your infrastructure</h2>
            <p className="text-neutral-400 max-w-md mb-8">
              CYNIC Cloud requires a Solana wallet signature to authenticate and grant access to your bare-metal containers.
            </p>
            <WalletMultiButton className="!bg-purple-600 hover:!bg-purple-700 transition-colors !rounded-lg" />
          </div>
        )}

        {/* Connected but not signed state */}
        {connected && publicKey && !isAuthenticated && (
          <div className="flex flex-col items-center justify-center py-24 text-center border border-neutral-800 rounded-2xl bg-neutral-900/50 backdrop-blur-sm shadow-2xl shadow-emerald-900/10 transition-all">
            <div className="h-16 w-16 bg-emerald-500/10 rounded-full flex items-center justify-center mb-6">
              <KeyRound className="h-8 w-8 text-emerald-500" />
            </div>
            <h2 className="text-2xl font-semibold mb-3 text-neutral-200">Unlock your Infrastructure</h2>
            <p className="text-neutral-400 max-w-md mb-8 leading-relaxed">
              Wallet connected as <span className="font-mono text-neutral-300 bg-neutral-800 px-2 py-1 rounded">{publicKey.toBase58().slice(0, 4)}...{publicKey.toBase58().slice(-4)}</span>.<br/>
              Please sign the authentication message to prove ownership.
            </p>
            <button 
              onClick={handleSign} 
              disabled={isSigning}
              className="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 disabled:hover:bg-emerald-600 text-white py-3 px-8 rounded-lg font-medium transition-colors shadow-lg shadow-emerald-500/20 flex items-center gap-2"
            >
              {isSigning ? (
                <>
                  <RefreshCw className="h-5 w-5 animate-spin" />
                  Waiting for Signature...
                </>
              ) : (
                <>
                  <KeyRound className="h-5 w-5" />
                  Sign to Unlock
                </>
              )}
            </button>
          </div>
        )}

        {/* Connected & Authenticated state */}
        {connected && publicKey && isAuthenticated && (
          <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div className="flex items-center justify-between border border-neutral-800 bg-neutral-900/50 p-4 rounded-xl">
              <div className="flex items-center gap-3 text-sm text-neutral-400">
                <span className="w-2 h-2 rounded-full bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]"></span>
                Authenticated as: <span className="font-mono text-neutral-300">{publicKey.toBase58().slice(0, 8)}...{publicKey.toBase58().slice(-8)}</span>
              </div>
            </div>

            {/* Service Card */}
            <div className="border border-neutral-800 rounded-2xl bg-neutral-900/50 overflow-hidden backdrop-blur-sm transition-all hover:border-neutral-700 shadow-xl">
              <div className="p-6 border-b border-neutral-800">
                <div className="flex justify-between items-start mb-4">
                  <div>
                    <div className="flex items-center gap-3 mb-1">
                      <h3 className="text-xl font-bold text-neutral-100">Robinhood (ASDev)</h3>
                      <span className="px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                        Running
                      </span>
                    </div>
                    <p className="text-sm text-neutral-400">Node.js API • LXC Container 200</p>
                  </div>
                  <button className="p-2 bg-neutral-800 hover:bg-neutral-700 rounded-lg transition-colors border border-neutral-700">
                    <Code2 className="h-5 w-5 text-neutral-300" />
                  </button>
                </div>
                
                <div className="flex gap-4 mt-8">
                  <button 
                    onClick={() => handleAction("deploy_asdf")}
                    disabled={isSigning}
                    className="flex-1 flex items-center justify-center gap-2 bg-purple-600 hover:bg-purple-700 text-white py-2.5 rounded-lg font-medium transition-colors shadow-lg shadow-purple-600/20 disabled:opacity-50">
                    <RefreshCw className={`h-4 w-4 ${isSigning ? 'animate-spin' : ''}`} />
                    {isSigning ? 'Deploying...' : 'Deploy Latest Commit'}
                  </button>
                  <button className="flex items-center justify-center gap-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-200 py-2.5 px-6 rounded-lg font-medium border border-neutral-700 transition-colors">
                    <Terminal className="h-4 w-4" />
                    View Logs
                  </button>
                </div>
              </div>
              
              {/* Fake terminal logs for the MVP look */}
              <div className="bg-black/60 p-4 font-mono text-xs text-neutral-400 h-48 overflow-y-auto">
                <div className="text-neutral-500 mb-2"># Latest container logs for asdf-host_asdev_1</div>
                <div>[Info] Starting Node.js application...</div>
                <div>[Info] Loading environment variables from .env</div>
                <div>[Info] Connected to Helius RPC endpoint</div>
                <div className="text-emerald-400">[Success] ASDev Server listening on port 3000</div>
                <div>[Request] GET /api/v1/health - 200 OK (2ms)</div>
                <div>[Request] GET /api/v1/health - 200 OK (1ms)</div>
                <div className="animate-pulse mt-2">_</div>
              </div>
            </div>

            {/* Freebox Organ Card */}
            <div className="border border-neutral-800 rounded-2xl bg-neutral-900/50 overflow-hidden backdrop-blur-sm transition-all hover:border-neutral-700 shadow-xl mt-6">
              <div className="p-6 border-b border-neutral-800">
                <div className="flex justify-between items-start mb-4">
                  <div>
                    <div className="flex items-center gap-3 mb-1">
                      <h3 className="text-xl font-bold text-neutral-100">Freebox Gateway</h3>
                      <span className="px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                        Active
                      </span>
                    </div>
                    <p className="text-sm text-neutral-400">organ-freebox • Host Execution</p>
                  </div>
                  <button className="p-2 bg-neutral-800 hover:bg-neutral-700 rounded-lg transition-colors border border-neutral-700">
                    <Server className="h-5 w-5 text-neutral-300" />
                  </button>
                </div>
                
                <div className="flex gap-4 mt-8">
                  <button 
                    onClick={() => handleAction("open_freebox")}
                    disabled={isSigning}
                    className="flex-1 flex items-center justify-center gap-2 bg-blue-600 hover:bg-blue-700 text-white py-2.5 rounded-lg font-medium transition-colors shadow-lg shadow-blue-600/20 disabled:opacity-50">
                    <RefreshCw className={`h-4 w-4 ${isSigning ? 'animate-spin' : ''}`} />
                    {isSigning ? 'Opening...' : 'Open Web Port (443)'}
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </main>
  );
}
