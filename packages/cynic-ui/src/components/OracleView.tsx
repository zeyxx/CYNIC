import { useState, useEffect } from 'react';
import { getObservations, getAgentTasks, updateAgentTaskResult } from '../api';
import type { Observation, AgentTask } from '../types';

interface Props {
  isAdmin: boolean;
}

export function OracleView({ isAdmin }: Props) {
  const [feed, setFeed] = useState<Observation[]>([]);
  const [tasks, setTasks] = useState<AgentTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingTask, setEditingTask] = useState<{ taskId: string; text: string } | null>(null);

  const loadData = async () => {
    try {
      const [obs, t] = await Promise.all([
        getObservations('community', 50),
        getAgentTasks('community-manager', 50)
      ]);
      setFeed(obs);
      setTasks(t.tasks || []);
    } catch (e) {
      console.error('Failed to load oracle data', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    const timer = setInterval(loadData, 10000);
    return () => clearInterval(timer);
  }, []);

  const handleApprove = async (task: AgentTask) => {
    if (!isAdmin) return;
    try {
      const cleanResult = (task.result || '').replace('[DRAFT] ', '');
      await updateAgentTaskResult(task.id, cleanResult);
      loadData();
    } catch (e) {
      alert('Failed to approve draft');
    }
  };

  const handleSaveEdit = async () => {
    if (!editingTask || !isAdmin) return;
    try {
      await updateAgentTaskResult(editingTask.taskId, editingTask.text);
      setEditingTask(null);
      loadData();
    } catch (e) {
      alert('Failed to save edit');
    }
  };

  if (loading && feed.length === 0) {
    return <div style={{ color: 'var(--text-dim)', padding: 20, fontFamily: 'var(--font-mono)', fontSize: 12 }}>Initializing social cortex...</div>;
  }

  const pendingTasks = tasks.filter(t => t.error === 'awaiting_human_approval');

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      {/* ── ALIGNMENT GATE (Sovereign Tier) ── */}
      {pendingTasks.length > 0 && (
        <div style={{
          background: 'var(--gold-glow)',
          border: '1px solid var(--border-bright)',
          borderRadius: 12,
          padding: 20,
        }}>
          <h3 style={{ margin: '0 0 16px 0', fontSize: 13, color: 'var(--gold)', letterSpacing: 2 }}>
            {isAdmin ? 'PENDING APPROVALS' : 'ORACLE IS THINKING...'}
          </h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            {pendingTasks.map((task) => {
              const content = JSON.parse(task.content);
              return (
                <div key={task.id} style={{
                  background: 'var(--card)',
                  border: '1px solid var(--border)',
                  borderRadius: 8,
                  padding: 16,
                }}>
                  <div style={{ display: 'flex', justifySelf: 'space-between', marginBottom: 12 }}>
                    <span style={{ fontSize: 11, color: 'var(--text-dim)', fontFamily: 'var(--font-mono)' }}>
                      CHAT_ID: {content.chat_id}
                    </span>
                    <span style={{ 
                      fontSize: 10, 
                      color: 'var(--gold)', 
                      background: 'rgba(201, 168, 76, 0.1)', 
                      padding: '2px 6px', 
                      borderRadius: 4,
                      marginLeft: 'auto'
                    }}>
                      AWAITING_ALIGNMENT
                    </span>
                  </div>
                  
                  <div style={{ 
                    color: 'var(--text-dim)', 
                    fontSize: 13, 
                    marginBottom: 12,
                    padding: '8px 12px',
                    borderLeft: '2px solid var(--gold-dim)',
                    background: 'rgba(0,0,0,0.2)'
                  }}>
                    {content.question}
                  </div>

                  {isAdmin ? (
                    editingTask?.taskId === task.id ? (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                        <textarea
                          value={editingTask.text}
                          onChange={(e) => setEditingTask({ ...editingTask, text: e.target.value })}
                          style={{
                            background: '#000',
                            color: '#fff',
                            border: '1px solid var(--gold)',
                            borderRadius: 4,
                            padding: 10,
                            fontSize: 13,
                            minHeight: 80,
                            fontFamily: 'inherit',
                          }}
                        />
                        <div style={{ display: 'flex', gap: 8 }}>
                          <button 
                            onClick={handleSaveEdit}
                            style={{ background: 'var(--gold)', color: '#000', border: 'none', padding: '6px 12px', borderRadius: 4, fontSize: 11, fontWeight: 'bold', cursor: 'pointer' }}
                          >
                            SAVE & SEND
                          </button>
                          <button 
                            onClick={() => setEditingTask(null)}
                            style={{ background: 'transparent', color: 'var(--text-dim)', border: '1px solid var(--border-bright)', padding: '6px 12px', borderRadius: 4, fontSize: 11, cursor: 'pointer' }}
                          >
                            CANCEL
                          </button>
                        </div>
                      </div>
                    ) : (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                        <div style={{ color: 'var(--text)', fontSize: 13, fontStyle: 'italic', opacity: 0.8 }}>
                          Oracle Draft: {task.result?.replace('[DRAFT] ', '')}
                        </div>
                        <div style={{ display: 'flex', gap: 8 }}>
                          <button 
                            onClick={() => handleApprove(task)}
                            style={{ background: 'rgba(201, 168, 76, 0.15)', color: 'var(--gold)', border: '1px solid var(--gold)', padding: '6px 12px', borderRadius: 4, fontSize: 11, fontWeight: 'bold', cursor: 'pointer' }}
                          >
                            APPROVE
                          </button>
                          <button 
                            onClick={() => setEditingTask({ taskId: task.id, text: task.result?.replace('[DRAFT] ', '') || '' })}
                            style={{ background: 'transparent', color: 'var(--text-dim)', border: '1px solid var(--border-bright)', padding: '6px 12px', borderRadius: 4, fontSize: 11, cursor: 'pointer' }}
                          >
                            EDIT
                          </button>
                        </div>
                      </div>
                    )
                  ) : (
                    <div style={{ color: 'var(--text-muted)', fontSize: 12, fontStyle: 'italic' }}>
                      [Draft hidden from public view until alignment confirmed]
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* ── SOCIAL FEED (Public/Community Tier) ── */}
      <div style={{
        background: 'var(--surface)',
        border: '1px solid var(--border)',
        borderRadius: 12,
        padding: 20,
      }}>
        <h3 style={{ margin: '0 0 16px 0', fontSize: 13, color: 'var(--text-dim)', letterSpacing: 2 }}>
          SOCIAL FEED (COMMUNITY)
        </h3>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {feed.map((obs, i) => {
            const isOracle = obs.agent_id === 'telegram-bot' || obs.agent_id === 'hermes-telegram';
            const user = obs.value?.username || obs.agent_id;
            const content = obs.context || obs.value?.text || '';
            
            return (
              <div key={i} style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: isOracle ? 'flex-end' : 'flex-start',
                gap: 4,
              }}>
                <div style={{ display: 'flex', gap: 8, fontSize: 10, color: 'var(--text-muted)', marginBottom: 2, fontFamily: 'var(--font-mono)' }}>
                  <span>{user}</span>
                  <span>•</span>
                  <span>{new Date(obs.timestamp || '').toLocaleTimeString()}</span>
                </div>
                <div style={{
                  maxWidth: '80%',
                  padding: '10px 14px',
                  borderRadius: 12,
                  borderTopLeftRadius: isOracle ? 12 : 2,
                  borderTopRightRadius: isOracle ? 2 : 12,
                  background: isOracle ? 'rgba(201, 168, 76, 0.1)' : 'var(--card)',
                  border: isOracle ? '1px solid rgba(201, 168, 76, 0.2)' : '1px solid var(--border)',
                  color: isOracle ? 'var(--gold)' : 'var(--text)',
                  fontSize: 13,
                  lineHeight: 1.5,
                }}>
                  {content}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
