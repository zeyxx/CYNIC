import { useState, useCallback } from 'react';
import { Chessboard } from 'react-chessboard';
import type { PieceDropHandlerArgs } from 'react-chessboard';
import { Chess } from 'chess.js';
import { judgeContent } from '../api';
import type { Verdict } from '../types';

interface Props {
  onVerdict: (v: Verdict) => void;
  onLoading: (l: boolean) => void;
}

export function ChessTab({ onVerdict, onLoading }: Props) {
  const [game, setGame] = useState(new Chess());
  const [lastMove, setLastMove] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [judgedMoves, setJudgedMoves] = useState(0);

  const onDrop = useCallback(
    ({ sourceSquare, targetSquare }: PieceDropHandlerArgs): boolean => {
      if (!targetSquare) return false;
      const gameCopy = new Chess(game.fen());
      let move;
      try {
        move = gameCopy.move({ from: sourceSquare, to: targetSquare, promotion: 'q' });
      } catch {
        return false;
      }
      if (!move) return false;

      setGame(gameCopy);
      setLastMove(move.san);
      setJudgedMoves(n => n + 1);
      setError(null);

      const prevFen = game.fen();
      const newFen = gameCopy.fen();
      const side = move.color === 'w' ? 'White' : 'Black';
      const moveNum = gameCopy.moveNumber();

      onLoading(true);
      judgeContent({
        content: `Chess position FEN: ${newFen}`,
        context: `${side} played ${move.san} (move ${moveNum}). Previous position: ${prevFen}`,
        domain: 'chess',
      })
        .then(v => { onVerdict(v); onLoading(false); })
        .catch(e => { setError(e.message); onLoading(false); });

      return true;
    },
    [game, onVerdict, onLoading]
  );

  const reset = () => {
    setGame(new Chess());
    setLastMove(null);
    setJudgedMoves(0);
    setError(null);
  };

  return (
    <div className="chess-tab">
      <div className="chess-board-wrap">
        <Chessboard
          options={{
            position: game.fen(),
            onPieceDrop: onDrop,
            darkSquareStyle: { backgroundColor: '#1c1c1c' },
            lightSquareStyle: { backgroundColor: '#2e2e2e' },
            boardStyle: {
              borderRadius: '4px',
              boxShadow: '0 0 24px rgba(255, 215, 0, 0.06)',
              width: '400px',
            },
          }}
        />
      </div>

      <div className="chess-info">
        {lastMove ? (
          <div className="last-move">
            <span className="info-label">last move</span>
            <span className="info-value">{lastMove}</span>
          </div>
        ) : (
          <p className="chess-hint">Play a move — CYNIC judges automatically</p>
        )}

        {judgedMoves > 0 && (
          <div className="last-move">
            <span className="info-label">moves judged</span>
            <span className="info-value">{judgedMoves}</span>
          </div>
        )}

        {error && <p className="error-msg">{error}</p>}

        <button className="reset-btn" onClick={reset}>
          Reset board
        </button>
      </div>

      <div className="fen-display">
        <span className="info-label">position</span>
        <code className="fen-text">{game.fen()}</code>
      </div>
    </div>
  );
}
