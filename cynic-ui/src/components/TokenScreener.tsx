import { useState, useCallback } from 'react';
import { DeliberationView } from './DeliberationView';
import type { JudgeRequest, Verdict } from '../types';
import { getSelectedDogs } from '../utils';

export function TokenScreener() {
  const [input, setInput] = useState('');
  const [request, setRequest] = useState<JudgeRequest | null>(null);
  const [lastVerdict, setLastVerdict] = useState<Verdict | null>(null);

  const submit = useCallback(() => {
    const trimmed = input.trim();
    if (!trimmed) return;

    setLastVerdict(null);
    setRequest({
      content: trimmed,
      domain: 'token-analysis',
      dogs: getSelectedDogs(),
    });
  }, [input]);

  const handleComplete = useCallback((v: Verdict) => {
    setLastVerdict(v);
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      {/* Input area */}
      <div style={{
        display: 'flex',
        gap: 10,
        alignItems: 'stretch',
      }}>
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
          placeholder="Paste Solana address or describe a token..."
          style={{
            flex: 1,
            padding: '12px 16px',
            background: '#111',
            border: '1px solid #222',
            borderRadius: 8,
            color: '#e0e0e0',
            fontFamily: 'monospace',
            fontSize: 14,
            outline: 'none',
          }}
        />
        <button
          onClick={submit}
          disabled={!input.trim()}
          style={{
            padding: '12px 24px',
            background: input.trim() ? '#C9A84C' : '#222',
            color: input.trim() ? '#0a0a0a' : '#444',
            border: 'none',
            borderRadius: 8,
            fontFamily: 'monospace',
            fontSize: 13,
            fontWeight: 900,
            letterSpacing: 2,
            cursor: input.trim() ? 'pointer' : 'default',
            transition: 'all 0.2s',
          }}
        >
          SCREEN
        </button>
      </div>

      {/* Quick examples */}
      <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
        {[
          { label: 'JUP', value: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN' },
          { label: 'BONK', value: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263' },
          { label: 'SOL', value: 'So11111111111111111111111111111111111111112' },
        ].map((ex) => (
          <button
            key={ex.label}
            onClick={() => { setInput(ex.value); }}
            style={{
              padding: '4px 10px',
              background: '#1a1a1a',
              border: '1px solid #2a2a2a',
              borderRadius: 4,
              color: '#666',
              fontFamily: 'monospace',
              fontSize: 11,
              cursor: 'pointer',
            }}
          >
            {ex.label}
          </button>
        ))}
      </div>

      {/* Deliberation */}
      <DeliberationView request={request} onComplete={handleComplete} />

      {/* Verdict ID for settlement link (Phase 4) */}
      {lastVerdict && (
        <div style={{
          fontFamily: 'monospace',
          fontSize: 11,
          color: '#333',
          textAlign: 'center',
        }}>
          verdict:{lastVerdict.verdict_id}
        </div>
      )}
    </div>
  );
}
