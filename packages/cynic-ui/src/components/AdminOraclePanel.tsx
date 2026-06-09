import { useEffect, useState } from 'react';
import { dispatchAgentTask, getAgentTasks, updateAgentTaskResult } from '../api';
import type { AgentTask } from '../types';
import { SurfacePanel } from './SurfacePanel';

interface Props {
  isAdmin: boolean;
}

interface ProposalRecommendation {
  severity?: string;
  kind?: string;
  target?: string;
  action?: string;
}

interface ProposalContent {
  title?: string;
  summary?: string;
  source?: string;
  review_mode?: string;
  current_branch?: string;
  dirty_count?: number;
  stash_count?: number;
  open_pr_count?: number;
  recommendations?: ProposalRecommendation[];
}

function parseProposalContent(task: AgentTask): ProposalContent | null {
  try {
    const parsed = JSON.parse(task.content);
    return parsed && typeof parsed === 'object' ? (parsed as ProposalContent) : null;
  } catch {
    return null;
  }
}

function decisionPayload(decision: 'approved' | 'denied', note?: string) {
  return JSON.stringify({
    decision,
    reviewed_at: new Date().toISOString(),
    note: note || undefined,
  });
}

function buildRemediationTask(task: AgentTask, proposal: ProposalContent, note?: string) {
  return {
    kind: 'hermes',
    domain: 'organ-anvil',
    agent_id: 'organ-anvil-admin',
    content: JSON.stringify({
      objective: 'Organ Anvil approved remediation',
      proposal_task_id: task.id,
      review_mode: 'approval_required',
      reviewed_at: new Date().toISOString(),
      review_note: note || undefined,
      source: proposal.source || 'organ-anvil-proposal',
      current_branch: proposal.current_branch || undefined,
      dirty_count: proposal.dirty_count || 0,
      stash_count: proposal.stash_count || 0,
      open_pr_count: proposal.open_pr_count || 0,
      recommendations: proposal.recommendations || [],
      targets: [
        'infra/organ-anvil/state.json',
        'infra/organ-anvil/poh.json',
        'infra/organ-anvil/audit.jsonl',
        '.handoff.md',
      ],
      actions: [
        'Read the approved proposal and choose the smallest reversible remediation.',
        'Apply only the approved lifecycle action(s) for the current scope.',
        'Record the remediation result in the audit and handoff trail.',
        'Stop immediately if the action would widen scope beyond the approved proposal.',
      ],
    }),
  };
}

export function AdminOraclePanel({ isAdmin }: Props) {
  const [tasks, setTasks] = useState<AgentTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingTask, setEditingTask] = useState<{ taskId: string; note: string } | null>(null);

  const loadData = async () => {
    try {
      const t = isAdmin ? await getAgentTasks('organ-anvil-proposal', 50) : { tasks: [] };
      setTasks(t.tasks || []);
    } catch (e) {
      console.error('Failed to load admin oracle data', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    const timer = setInterval(loadData, 10000);
    return () => clearInterval(timer);
  }, [isAdmin]);

  const handleApprove = async (task: AgentTask, note?: string) => {
    if (!isAdmin) return;
    try {
      const proposal = parseProposalContent(task);
      await updateAgentTaskResult(task.id, decisionPayload('approved', note));
      if (proposal) {
        await dispatchAgentTask(buildRemediationTask(task, proposal, note));
      }
      setEditingTask(null);
      loadData();
    } catch (e) {
      alert('Failed to approve proposal');
    }
  };

  const handleDeny = async (task: AgentTask) => {
    if (!isAdmin) return;
    try {
      await updateAgentTaskResult(task.id, undefined, 'human_denied');
      setEditingTask(null);
      loadData();
    } catch (e) {
      alert('Failed to deny proposal');
    }
  };

  const handleSaveEdit = async () => {
    if (!editingTask || !isAdmin) return;
    const task = tasks.find((entry) => entry.id === editingTask.taskId);
    if (!task) return;
    try {
      const proposal = parseProposalContent(task);
      const note = editingTask.note.trim() || undefined;
      await updateAgentTaskResult(task.id, decisionPayload('approved', note));
      if (proposal) {
        await dispatchAgentTask(buildRemediationTask(task, proposal, note));
      }
      setEditingTask(null);
      loadData();
    } catch (e) {
      alert('Failed to save review note');
    }
  };

  if (!isAdmin) return null;

  return (
    <SurfacePanel
      eyebrow="ADMIN REVIEW"
      title="Remediation queue"
      subtitle="Approve or deny a proposal. Approving dispatches the follow-up remediation task immediately."
      actions={<span className="status-chip is-muted">{tasks.length} PENDING</span>}
    >
      {loading && tasks.length === 0 ? (
        <div className="loading-state">Loading review queue...</div>
      ) : tasks.length === 0 ? (
        <div className="empty-state">No fix proposals pending.</div>
      ) : (
        <div className="review-list">
          {tasks.map((task) => {
            const proposal = parseProposalContent(task);
            const recommendations = proposal?.recommendations ?? [];
            const summary = proposal?.summary ?? task.content;
            const title = proposal?.title ?? 'Organ Anvil proposal';

            return (
              <article key={task.id} className="review-card">
                <div className="review-head">
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                    <div className="review-meta">
                      <span className="pill is-muted">TASK {task.id}</span>
                      <span className="pill is-gold">{task.status.toUpperCase()}</span>
                    </div>
                    <div className="review-title">{title}</div>
                    <div className="review-sub">{task.domain} · {task.kind} · {task.agent_id ?? 'n/a'}</div>
                  </div>
                  <span className="review-status">AWAITING REVIEW</span>
                </div>

                <div className="review-summary">{summary}</div>

                {proposal ? (
                  <div className="review-tags">
                    {typeof proposal.dirty_count === 'number' ? <span className="pill is-muted">dirty {proposal.dirty_count}</span> : null}
                    {typeof proposal.stash_count === 'number' ? <span className="pill is-muted">stashes {proposal.stash_count}</span> : null}
                    {typeof proposal.open_pr_count === 'number' ? <span className="pill is-muted">prs {proposal.open_pr_count}</span> : null}
                    {proposal.current_branch ? <span className="pill is-muted">{proposal.current_branch}</span> : null}
                  </div>
                ) : null}

                {recommendations.length > 0 ? (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                    <div className="section-label">RECOMMENDATIONS</div>
                    {recommendations.map((rec, index) => (
                      <div key={`${task.id}-${index}`} className="surface-card" style={{ minHeight: 0 }}>
                        <div className="meta-line">
                          {rec.severity ? <span className="pill is-muted">{rec.severity}</span> : null}
                          {rec.kind ? <span className="pill is-muted">{rec.kind}</span> : null}
                          {rec.target ? <span className="pill is-muted">{rec.target}</span> : null}
                        </div>
                        <div className="oracle-message">{rec.action}</div>
                      </div>
                    ))}
                  </div>
                ) : null}

                {editingTask?.taskId === task.id ? (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                    <textarea
                      value={editingTask.note}
                      onChange={(e) => setEditingTask({ ...editingTask, note: e.target.value })}
                      className="review-editor"
                      placeholder="Review note"
                    />
                    <div className="review-actions">
                      <button onClick={handleSaveEdit} className="button button-primary">
                        APPROVE NOTE
                      </button>
                      <button onClick={() => setEditingTask(null)} className="button button-ghost">
                        CANCEL
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="review-actions">
                    <button onClick={() => handleApprove(task)} className="button button-primary">
                      APPROVE
                    </button>
                    <button onClick={() => handleDeny(task)} className="button button-ghost">
                      DENY
                    </button>
                    <button onClick={() => setEditingTask({ taskId: task.id, note: proposal?.summary ?? '' })} className="button button-ghost">
                      EDIT NOTE
                    </button>
                  </div>
                )}
              </article>
            );
          })}
        </div>
      )}
    </SurfacePanel>
  );
}
