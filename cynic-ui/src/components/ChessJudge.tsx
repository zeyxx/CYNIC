import { useState, useCallback } from 'react';
import { Chessboard } from 'react-chessboard';
import { Chess } from 'chess.js';
import { judge } from '../api';
import type { Verdict } from '../types';
import { VerdictDisplay } from './VerdictDisplay';

interface Props {
  onVerdict?: (verdict: Verdict) => void;
}

export function ChessJudge({ onVerdict }: Props) {
  const [game, setGame] = useState(new Chess());
  const [fen, setFen] = useState(game.fen());
  const [lastMove, setLastMove] = useState<string | null>(null);
  const [verdict, setVerdict] = useState<Verdict | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [moveHistory, setMoveHistory] = useState<string[]>([]);

  const makeMove = useCallback(async (sourceSquare: string, targetSquare: string) => {
    const gameCopy = new Chess(game.fen());
    let move;
    try {
      move = gameCopy.move({ from: sourceSquare, to: targetSquare, promotion: 'q' });
    } catch {
      return false;
    }
    if (!move) return false;

    setGame(gameCopy);
    setFen(gameCopy.fen());
    const history = gameCopy.history();
    setMoveHistory(history);
    const moveStr = move.san;
    setLastMove(moveStr);
    setError(null);

    const moveNum = Math.ceil(history.length / 2);
    const content = `Chess move: ${moveStr} (move ${moveNum}). Position after move: ${gameCopy.fen()}. Full game: ${history.join(' ')}`;

    setLoading(true);
    try {
      const result = await judge({ content, domain: 'chess', context: `Move ${moveNum}: ${moveStr}` });
      setVerdict(result);
      onVerdict?.(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to judge move');
    } finally {
      setLoading(false);
    }
    return true;
  }, [game, onVerdict]);

  const resetGame = () => {
    const newGame = new Chess();
    setGame(newGame);
    setFen(newGame.fen());
    setLastMove(null);
    setVerdict(null);
    setError(null);
    setMoveHistory([]);
  };

  return (
    <div style={{ display: 'flex', gap: 24, flexWrap: 'wrap' }}>
      <div style={{ flex: '0 0 auto' }}>
        <div style={{ marginBottom: 12, display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div>
            {lastMove && (
              <span style={{ color: '#C9A84C', fontFamily: 'monospace', fontSize: 14 }}>
                Last: {lastMove}
              </span>
            )}
          </div>
          <button
            onClick={resetGame}
            style={{
              background: '#222',
              color: '#aaa',
              border: '1px solid #444',
              borderRadius: 6,
              padding: '6px 14px',
              cursor: 'pointer',
              fontSize: 12,
            }}
          >
            Reset Board
          </button>
        </div>
        <div style={{ width: 400, borderRadius: 8, overflow: 'hidden', border: '2px solid #333' }}>
          <Chessboard
            position={fen}
            onPieceDrop={makeMove}
            boardWidth={400}
            customDarkSquareStyle={{ backgroundColor: '#2d3748' }}
            customLightSquareStyle={{ backgroundColor: '#718096' }}
          />
        </div>
        {moveHistory.length > 0 && (
          <div style={{
            marginTop: 12,
            padding: 10,
            background: '#111',
            borderRadius: 8,
            border: '1px solid #222',
            fontFamily: 'monospace',
            fontSize: 12,
            color: '#888',
            maxHeight: 80,
            overflowY: 'auto',
          }}>
            {moveHistory.map((m, i) => (
              <span key={i}>
                {i % 2 === 0 && <span style={{ color: '#555' }}>{Math.floor(i/2)+1}. </span>}
                <span style={{ color: i === moveHistory.length - 1 ? '#C9A84C' : '#888' }}>{m} </span>
              </span>
            ))}
          </div>
        )}
        {game.isCheckmate() && (
          <div style={{ textAlign: 'center', padding: 10, color: '#F44336', fontWeight: 700, marginTop: 8 }}>
            ♛ CHECKMATE
          </div>
        )}
        {game.isDraw() && (
          <div style={{ textAlign: 'center', padding: 10, color: '#FF9800', fontWeight: 700, marginTop: 8 }}>
            ½-½ DRAW
          </div>
        )}
      </div>

      <div style={{ flex: 1, minWidth: 300 }}>
        {loading && (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            padding: 40,
            color: '#C9A84C',
          }}>
            <div style={{ fontSize: 32, marginBottom: 12, animation: 'spin 1s linear infinite' }}>🌀</div>
            <div style={{ fontFamily: 'monospace', letterSpacing: 2 }}>JUDGING...</div>
            <div style={{ fontSize: 12, color: '#555', marginTop: 8 }}>Dogs evaluating position...</div>
          </div>
        )}
        {error && (
          <div style={{
            padding: 16,
            background: '#F4433611',
            border: '1px solid #F44336',
            borderRadius: 8,
            color: '#F44336',
            fontSize: 13,
          }}>
            ⚠ {error}
          </div>
        )}
        {verdict && !loading && <VerdictDisplay verdict={verdict} />}
        {!verdict && !loading && !error && (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            padding: 60,
            color: '#444',
            textAlign: 'center',
          }}>
            <div style={{ fontSize: 48, marginBottom: 16 }}>♟</div>
            <div style={{ fontFamily: 'monospace', letterSpacing: 2, fontSize: 14 }}>
              MAKE A MOVE
            </div>
            <div style={{ fontSize: 12, marginTop: 8 }}>
              Each move will be judged by the CYNIC Dogs
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
