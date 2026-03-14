import { useState, useCallback, useMemo } from 'react';
import { Chessboard } from 'react-chessboard';
import { Chess } from 'chess.js';
import type { Square } from 'chess.js';
import { judge } from '../api';
import type { Verdict } from '../types';
import { VerdictDisplay } from './VerdictDisplay';

const LIGHT = '#b0bec5';
const DARK = '#37474f';
const HIGHLIGHT_MOVE = 'rgba(201,168,76,0.5)';
const HIGHLIGHT_SELECTED = 'rgba(201,168,76,0.8)';
const HIGHLIGHT_CHECK = 'rgba(244,67,54,0.6)';
const HIGHLIGHT_LAST_FROM = 'rgba(100,181,246,0.35)';
const HIGHLIGHT_LAST_TO = 'rgba(100,181,246,0.55)';

type PromotionPiece = 'q' | 'r' | 'b' | 'n';

interface Props {
  onVerdict?: (verdict: Verdict) => void;
}

function getGameStatus(game: Chess): { label: string; color: string; over: boolean } {
  if (game.isCheckmate()) {
    const winner = game.turn() === 'w' ? 'Black' : 'White';
    return { label: `♛ CHECKMATE — ${winner} wins`, color: '#F44336', over: true };
  }
  if (game.isStalemate()) return { label: '⚡ STALEMATE — Draw', color: '#FF9800', over: true };
  if (game.isInsufficientMaterial()) return { label: '🔔 INSUFFICIENT MATERIAL — Draw', color: '#FF9800', over: true };
  if (game.isThreefoldRepetition()) return { label: '🔄 THREEFOLD REPETITION — Draw', color: '#FF9800', over: true };
  if (game.isDraw()) return { label: '½-½ DRAW', color: '#FF9800', over: true };
  if (game.isCheck()) {
    const inCheck = game.turn() === 'w' ? 'White' : 'Black';
    return { label: `⚠ CHECK — ${inCheck} king in check`, color: '#FF9800', over: false };
  }
  const turn = game.turn() === 'w' ? 'White' : 'Black';
  return { label: `${turn} to move`, color: game.turn() === 'w' ? '#f0f0f0' : '#aaa', over: false };
}

function getKingSquare(game: Chess, color: 'w' | 'b'): Square | null {
  for (const rank of ['1','2','3','4','5','6','7','8']) {
    for (const file of ['a','b','c','d','e','f','g','h']) {
      const sq = (file + rank) as Square;
      const piece = game.get(sq);
      if (piece && piece.type === 'k' && piece.color === color) return sq;
    }
  }
  return null;
}

export function ChessJudge({ onVerdict }: Props) {
  const [game, setGame] = useState(() => new Chess());
  const [history, setHistory] = useState<Chess[]>([]);
  const [selectedSquare, setSelectedSquare] = useState<Square | null>(null);
  const [lastMove, setLastMove] = useState<{ from: Square; to: Square } | null>(null);
  const [verdict, setVerdict] = useState<Verdict | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [orientation, setOrientation] = useState<'white' | 'black'>('white');
  const [pendingPromotion, setPendingPromotion] = useState<{ from: Square; to: Square } | null>(null);

  const status = useMemo(() => getGameStatus(game), [game]);

  const legalMovesFrom = useCallback((sq: Square) => {
    return game.moves({ square: sq, verbose: true });
  }, [game]);

  const customSquareStyles = useMemo(() => {
    const styles: Record<string, React.CSSProperties> = {};

    if (lastMove) {
      styles[lastMove.from] = { background: HIGHLIGHT_LAST_FROM };
      styles[lastMove.to] = { background: HIGHLIGHT_LAST_TO };
    }

    if (game.isCheck()) {
      const kingSquare = getKingSquare(game, game.turn());
      if (kingSquare) styles[kingSquare] = { background: HIGHLIGHT_CHECK };
    }

    if (selectedSquare) {
      styles[selectedSquare] = { background: HIGHLIGHT_SELECTED };
      const moves = legalMovesFrom(selectedSquare);
      for (const m of moves) {
        const target = m.to as Square;
        const isCapture = game.get(target) !== null;
        styles[target] = isCapture
          ? {
              background: 'radial-gradient(circle, rgba(244,67,54,0.5) 85%, transparent 85%)',
              borderRadius: '50%',
            }
          : {
              background: `radial-gradient(circle, ${HIGHLIGHT_MOVE} 28%, transparent 28%)`,
              borderRadius: '50%',
            };
      }
    }

    return styles;
  }, [selectedSquare, lastMove, game, legalMovesFrom]);

  const applyMove = useCallback(async (gameCopy: Chess, from: Square, to: Square, san: string) => {
    setHistory(prev => [...prev, game]);
    setGame(gameCopy);
    setLastMove({ from, to });
    setSelectedSquare(null);
    setError(null);

    const moves = gameCopy.history();
    const moveNum = Math.ceil(moves.length / 2);
    const content = `Chess move ${moveNum}: ${san}. Full game: ${moves.join(' ')}. FEN: ${gameCopy.fen()}`;

    setLoading(true);
    try {
      const result = await judge({ content, domain: 'chess', context: `Move ${moveNum}: ${san}` });
      setVerdict(result);
      onVerdict?.(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'API unreachable');
    } finally {
      setLoading(false);
    }
  }, [game, onVerdict]);

  const tryMove = useCallback((from: Square, to: Square, promotion?: PromotionPiece): boolean => {
    if (status.over) return false;
    const gameCopy = new Chess(game.fen());
    const legalMoves = gameCopy.moves({ square: from, verbose: true });
    const isLegal = legalMoves.some(m => m.to === to);
    if (!isLegal) return false;

    const needsPromotion = legalMoves.some(m => m.to === to && m.promotion);
    if (needsPromotion && !promotion) {
      setPendingPromotion({ from, to });
      return true;
    }

    let move;
    try {
      move = gameCopy.move({ from, to, promotion: promotion ?? 'q' });
    } catch { return false; }
    if (!move) return false;

    applyMove(gameCopy, from, to, move.san);
    return true;
  }, [game, status.over, applyMove]);

  const onSquareClick = useCallback((square: Square) => {
    if (status.over) return;

    if (selectedSquare) {
      if (selectedSquare === square) {
        setSelectedSquare(null);
        return;
      }
      const moved = tryMove(selectedSquare, square);
      if (!moved) {
        const piece = game.get(square);
        if (piece && piece.color === game.turn()) {
          setSelectedSquare(square);
        } else {
          setSelectedSquare(null);
        }
      }
    } else {
      const piece = game.get(square);
      if (piece && piece.color === game.turn()) {
        setSelectedSquare(square);
      }
    }
  }, [selectedSquare, status.over, tryMove, game]);

  const onPieceDrop = useCallback((from: string, to: string): boolean => {
    return tryMove(from as Square, to as Square);
  }, [tryMove]);

  const confirmPromotion = useCallback((piece: PromotionPiece) => {
    if (!pendingPromotion) return;
    tryMove(pendingPromotion.from, pendingPromotion.to, piece);
    setPendingPromotion(null);
  }, [pendingPromotion, tryMove]);

  const undoMove = useCallback(() => {
    if (history.length === 0) return;
    const prev = history[history.length - 1];
    setGame(prev);
    setHistory(h => h.slice(0, -1));
    setSelectedSquare(null);
    setVerdict(null);
    setError(null);
    const prevHistory = prev.history({ verbose: true });
    if (prevHistory.length > 0) {
      const last = prevHistory[prevHistory.length - 1];
      setLastMove({ from: last.from as Square, to: last.to as Square });
    } else {
      setLastMove(null);
    }
  }, [history]);

  const resetGame = useCallback(() => {
    setGame(new Chess());
    setHistory([]);
    setSelectedSquare(null);
    setLastMove(null);
    setVerdict(null);
    setError(null);
    setPendingPromotion(null);
  }, []);

  const moveList = game.history();
  const fenHistory = game.history({ verbose: true });

  return (
    <div style={{ display: 'flex', gap: 24, flexWrap: 'wrap', alignItems: 'flex-start' }}>
      <div style={{ flex: '0 0 auto' }}>
        <div style={{ marginBottom: 10, display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
          <div style={{
            padding: '6px 14px',
            borderRadius: 6,
            background: '#111',
            border: `1px solid ${status.over ? '#444' : '#222'}`,
            color: status.color,
            fontFamily: 'monospace',
            fontSize: 13,
            flex: 1,
          }}>
            {status.label}
          </div>
          <button onClick={undoMove} disabled={history.length === 0 || loading} style={btnStyle(history.length === 0 || loading)}>
            ↩ Undo
          </button>
          <button onClick={() => setOrientation(o => o === 'white' ? 'black' : 'white')} style={btnStyle(false)}>
            ⇅ Flip
          </button>
          <button onClick={resetGame} style={btnStyle(false)}>
            ↺ Reset
          </button>
        </div>

        <div style={{ position: 'relative', width: 440 }}>
          <Chessboard
            position={game.fen()}
            onPieceDrop={onPieceDrop}
            onSquareClick={onSquareClick}
            boardWidth={440}
            boardOrientation={orientation}
            customSquareStyles={customSquareStyles}
            customDarkSquareStyle={{ backgroundColor: DARK }}
            customLightSquareStyle={{ backgroundColor: LIGHT }}
            arePiecesDraggable={!status.over}
          />

          {pendingPromotion && (
            <div style={{
              position: 'absolute', inset: 0,
              background: 'rgba(0,0,0,0.85)',
              display: 'flex', flexDirection: 'column',
              alignItems: 'center', justifyContent: 'center',
              gap: 12, zIndex: 10,
              borderRadius: 4,
            }}>
              <div style={{ color: '#C9A84C', fontFamily: 'monospace', fontSize: 14, marginBottom: 8 }}>
                CHOOSE PROMOTION PIECE
              </div>
              <div style={{ display: 'flex', gap: 12 }}>
                {(['q','r','b','n'] as PromotionPiece[]).map(p => {
                  const labels: Record<PromotionPiece, string> = { q: '♛ Queen', r: '♜ Rook', b: '♝ Bishop', n: '♞ Knight' };
                  return (
                    <button key={p} onClick={() => confirmPromotion(p)} style={{
                      width: 80, height: 80,
                      background: '#1a1a1a',
                      border: '2px solid #C9A84C',
                      borderRadius: 8,
                      color: '#C9A84C',
                      fontSize: 13,
                      cursor: 'pointer',
                      fontFamily: 'monospace',
                      display: 'flex', flexDirection: 'column',
                      alignItems: 'center', justifyContent: 'center',
                      gap: 4,
                    }}>
                      <span style={{ fontSize: 28 }}>{labels[p].split(' ')[0]}</span>
                      <span style={{ fontSize: 11 }}>{labels[p].split(' ')[1]}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        <div style={{
          marginTop: 10,
          padding: '10px 12px',
          background: '#0a0a0a',
          borderRadius: 8,
          border: '1px solid #1a1a1a',
          fontFamily: 'monospace',
          fontSize: 12,
          minHeight: 52,
        }}>
          {moveList.length === 0 ? (
            <span style={{ color: '#333' }}>No moves yet</span>
          ) : (
            moveList.map((m, i) => (
              <span key={i}>
                {i % 2 === 0 && (
                  <span style={{ color: '#444', marginRight: 3 }}>{Math.floor(i / 2) + 1}.</span>
                )}
                <span style={{
                  color: i === moveList.length - 1 ? '#C9A84C' : '#777',
                  marginRight: 6,
                }}>
                  {m}
                </span>
              </span>
            ))
          )}
        </div>

        <div style={{
          marginTop: 8,
          padding: '6px 10px',
          background: '#0a0a0a',
          borderRadius: 6,
          border: '1px solid #1a1a1a',
          fontFamily: 'monospace',
          fontSize: 10,
          color: '#333',
          wordBreak: 'break-all',
        }}>
          {fenHistory.length > 0 ? game.fen() : 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'}
        </div>
      </div>

      <div style={{ flex: 1, minWidth: 300 }}>
        {loading && (
          <div style={{
            display: 'flex', flexDirection: 'column',
            alignItems: 'center', justifyContent: 'center',
            padding: 40, color: '#C9A84C',
          }}>
            <div style={{ fontSize: 32, marginBottom: 12, animation: 'spin 1s linear infinite' }}>🌀</div>
            <div style={{ fontFamily: 'monospace', letterSpacing: 2 }}>JUDGING MOVE...</div>
            <div style={{ fontSize: 12, color: '#555', marginTop: 8 }}>Dogs evaluating the position...</div>
          </div>
        )}
        {error && (
          <div style={{
            padding: 16, background: '#F4433611',
            border: '1px solid #F44336', borderRadius: 8,
            color: '#F44336', fontSize: 13, marginBottom: 12,
          }}>
            ⚠ {error}
            <div style={{ fontSize: 11, color: '#F44336aa', marginTop: 6 }}>
              Check the kernel URL in Settings (top-right menu)
            </div>
          </div>
        )}
        {verdict && !loading && <VerdictDisplay verdict={verdict} />}
        {!verdict && !loading && !error && (
          <div style={{
            display: 'flex', flexDirection: 'column',
            alignItems: 'center', justifyContent: 'center',
            padding: 60, color: '#444', textAlign: 'center',
          }}>
            <div style={{ fontSize: 56, marginBottom: 16 }}>♟</div>
            <div style={{ fontFamily: 'monospace', letterSpacing: 2, fontSize: 14 }}>
              MAKE A MOVE
            </div>
            <div style={{ fontSize: 12, marginTop: 8, color: '#333', lineHeight: 1.6 }}>
              Click a piece then a square, or drag.<br />
              Each move is judged by the CYNIC Dogs.
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function btnStyle(disabled: boolean): React.CSSProperties {
  return {
    background: disabled ? '#111' : '#1a1a1a',
    color: disabled ? '#333' : '#aaa',
    border: '1px solid #333',
    borderRadius: 6,
    padding: '6px 12px',
    cursor: disabled ? 'not-allowed' : 'pointer',
    fontSize: 12,
    fontFamily: 'monospace',
    transition: 'all 0.15s',
  };
}
