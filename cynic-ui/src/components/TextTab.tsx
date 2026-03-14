import { useState } from 'react';
import { judgeContent } from '../api';
import type { Verdict } from '../types';

interface Props {
  onVerdict: (v: Verdict) => void;
  onLoading: (l: boolean) => void;
}

export function TextTab({ onVerdict, onLoading }: Props) {
  const [content, setContent] = useState('');
  const [context, setContext] = useState('');
  const [domain, setDomain] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async () => {
    if (!content.trim()) return;
    setError(null);
    onLoading(true);
    try {
      const v = await judgeContent({
        content: content.trim(),
        context: context.trim() || undefined,
        domain: domain.trim() || undefined,
      });
      onVerdict(v);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Request failed');
    } finally {
      onLoading(false);
    }
  };

  const handleKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) handleSubmit();
  };

  return (
    <div className="text-tab">
      <div className="field">
        <label className="field-label">Content <span className="required">*</span></label>
        <textarea
          className="field-textarea"
          placeholder="Enter any statement, claim, or idea to judge…"
          value={content}
          onChange={e => setContent(e.target.value)}
          onKeyDown={handleKey}
          rows={6}
        />
      </div>

      <div className="field">
        <label className="field-label">Context <span className="optional">optional</span></label>
        <input
          className="field-input"
          placeholder="e.g. Scientific debate, 2024"
          value={context}
          onChange={e => setContext(e.target.value)}
        />
      </div>

      <div className="field">
        <label className="field-label">Domain <span className="optional">optional</span></label>
        <input
          className="field-input"
          placeholder="e.g. science, philosophy, politics"
          value={domain}
          onChange={e => setDomain(e.target.value)}
        />
      </div>

      {error && <p className="error-msg">{error}</p>}

      <button
        className="judge-btn"
        onClick={handleSubmit}
        disabled={!content.trim()}
      >
        Judge  <kbd>⌘ Enter</kbd>
      </button>
    </div>
  );
}
