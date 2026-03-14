import { useState, useRef, useCallback, useMemo } from 'react';
import { Chessboard } from 'react-chessboard';
import { Chess } from 'chess.js';
import type { Square } from 'chess.js';
import { judge } from '../api';
import type { Verdict } from '../types';
import { VerdictDisplay } from './VerdictDisplay';

// ─── Constants ────────────────────────────────────────────────
const LIGHT = '#b0bec5';
const DARK = '#37474f';
const HIGHLIGHT_MOVE = 'rgba(201,168,76,0.45)';
const HIGHLIGHT_SELECTED = 'rgba(201,168,76,0.75)';
const HIGHLIGHT_CHECK = 'rgba(244,67,54,0.55)';
const HIGHLIGHT_LAST = 'rgba(100,181,246,0.4)';

type PromotionPiece = 'q' | 'r' | 'b' | 'n';

interface Props {
  onVerdict?: (verdict: Verdict) => void;
}

// ─── Status helper (pure, no hooks) ───────────────────────────
function gameStatus(g: Chess) {
  if (g.isCheckmate()) {
    const w = g.turn() === 'w' ? 'Black' : 'White';
    return { label: `♛ CHECKMATE — ${w} wins`, color: '#F44336', over: true };
  }
  if (g.isStalemate())           return { label: '⚡ STALEMATE — Draw',              color: '#FF9800', over: true };
  if (g.isInsufficientMaterial())return { label: '🔔 INSUFFICIENT MATERIAL — Draw',  color: '#FF9800', over: true };
  if (g.isThreefoldRepetition()) return { label: '🔄 THREEFOLD REPETITION — Draw',   color: '#FF9800', over: true };
  if (g.isDraw())                return { label: '½-½ DRAW',                          color: '#FF9800', over: true };
  if (g.isCheck()) {
    const side = g.turn() === 'w' ? 'White' : 'Black';
    return { label: `⚠ CHECK — ${side} in check`, color: '#FF9800', over: false };
  }
  const side = g.turn() === 'w' ? 'White' : 'Black';
  return { label: `${side} to move`, color: g.turn() === 'w' ? '#e0e0e0' : '#aaa', over: false };
}

function findKing(g: Chess, color: 'w' | 'b'): Square | null {
  const files = ['a','b','c','d','e','f','g','h'];
  const ranks = ['1','2','3','4','5','6','7','8'];
  for (const f of files) for (const r of ranks) {
    const sq = (f + r) as Square;
    const p = g.get(sq);
    if (p && p.type === 'k' && p.color === color) return sq;
  }
  return null;
}

// ─── Main component ───────────────────────────────────────────
export function ChessJudge({ onVerdict }: Props) {
  // The game object lives in a ref — mutation is direct, no stale closures
  const gameRef = useRef(new Chess());

  // FEN drives re-renders — updated after every state change
  const [fen, setFen] = useState<string>(() => gameRef.current.fen());

  // Undo stack: list of FEN strings
  const undoStack = useRef<string[]>([]);

  const [selectedSquare, setSelectedSquare] = useState<Square | null>(null);
  const [lastMove, setLastMove]             = useState<[Square, Square] | null>(null);
  const [orientation, setOrientation]       = useState<'white' | 'black'>('white');
  const [pendingPromotion, setPendingPromotion] = useState<{ from: Square; to: Square } | null>(null);

  const [verdict, setVerdict] = useState<Verdict | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError]     = useState<string | null>(null);

  // Derived from fen so it's always in sync with the actual game
  const status = useMemo(() => gameStatus(gameRef.current), [fen]);

  // ─── Core move executor ──────────────────────────────────────
  const executeMove = useCallback((from: Square, to: Square, promotion?: PromotionPiece) => {
    const g = gameRef.current;

    // Save undo snapshot before mutating
    undoStack.current.push(g.fen());

    const move = g.move({ from, to, promotion: promotion ?? 'q' });
    if (!move) {
      undoStack.current.pop();
      return false;
    }

    const newFen = g.fen();
    setFen(newFen);
    setLastMove([from, to]);
    setSelectedSquare(null);
    setError(null);

    // Async judge call — does NOT block move execution
    const history  = g.history();
    const moveNum  = Math.ceil(history.length / 2);
    const content  = `Chess move ${moveNum}: ${move.san}. Game: ${history.join(' ')}. FEN: ${newFen}`;

    setLoading(true);
    judge({ content, domain: 'chess', context: `Move ${moveNum}: ${move.san}` })
      .then(r => { setVerdict(r); onVerdict?.(r); })
      .catch(e => setError(e instanceof Error ? e.message : 'API unreachable'))
      .finally(() => setLoading(false));

    return true;
  }, [onVerdict]);

  // ─── Promotion check ─────────────────────────────────────────
  const tryMove = useCallback((from: Square, to: Square, promotion?: PromotionPiece): boolean => {
    if (status.over) return false;
    const g = gameRef.current;

    // Check legality without mutating
    const legal = g.moves({ square: from, verbose: true });
    if (!legal.some(m => m.to === to)) return false;

    const needsPromotion = legal.some(m => m.to === to && m.promotion);
    if (needsPromotion && !promotion) {
      setPendingPromotion({ from, to });
      return true; // tell board to keep piece in place visually
    }

    return executeMove(from, to, promotion);
  }, [status.over, executeMove]);

  // ─── react-chessboard callbacks ──────────────────────────────
  // Drag-and-drop: primary interaction mechanism
  const onPieceDrop = useCallback(({ sourceSquare, targetSquare }: { sourceSquare: string; targetSquare: string | null }): boolean => {
    setSelectedSquare(null);
    if (!targetSquare) return false;
    return tryMove(sourceSquare as Square, targetSquare as Square);
  }, [tryMove]);

  // Click-to-move: secondary mechanism (click piece → click destination)
  const onSquareClick = useCallback(({ square }: { square: string }) => {
    if (status.over) return;
    const g = gameRef.current;
    const sq = square as Square;

    if (selectedSquare && selectedSquare !== sq) {
      // Attempt to move selected → clicked
      const moved = tryMove(selectedSquare, sq);
      if (!moved) {
        // Not a valid destination — try selecting new piece
        const piece = g.get(sq);
        setSelectedSquare(piece && piece.color === g.turn() ? sq : null);
      }
      return;
    }

    // Select or deselect
    const piece = g.get(sq);
    setSelectedSquare(
      piece && piece.color === g.turn() && sq !== selectedSquare ? sq : null
    );
  }, [selectedSquare, status.over, tryMove]);

  // ─── Promotion confirm ───────────────────────────────────────
  const confirmPromotion = useCallback((piece: PromotionPiece) => {
    if (!pendingPromotion) return;
    const { from, to } = pendingPromotion;
    setPendingPromotion(null);
    executeMove(from, to, piece);
  }, [pendingPromotion, executeMove]);

  // ─── Board controls ──────────────────────────────────────────
  const undoMove = useCallback(() => {
    if (undoStack.current.length === 0) return;
    const prevFen = undoStack.current.pop()!;
    gameRef.current = new Chess(prevFen);
    setFen(prevFen);
    setSelectedSquare(null);
    setLastMove(null);
    setVerdict(null);
    setError(null);
  }, []);

  const resetGame = useCallback(() => {
    gameRef.current = new Chess();
    undoStack.current = [];
    setFen(gameRef.current.fen());
    setSelectedSquare(null);
    setLastMove(null);
    setVerdict(null);
    setError(null);
    setPendingPromotion(null);
  }, []);

  const flipBoard = useCallback(() => {
    setOrientation(o => o === 'white' ? 'black' : 'white');
  }, []);

  // ─── Square highlight styles ─────────────────────────────────
  const customSquareStyles = useMemo<Record<string, React.CSSProperties>>(() => {
    const styles: Record<string, React.CSSProperties> = {};
    const g = gameRef.current;

    if (lastMove) {
      styles[lastMove[0]] = { background: HIGHLIGHT_LAST };
      styles[lastMove[1]] = { background: HIGHLIGHT_LAST };
    }

    if (g.isCheck()) {
      const ks = findKing(g, g.turn());
      if (ks) styles[ks] = { background: HIGHLIGHT_CHECK };
    }

    if (selectedSquare) {
      styles[selectedSquare] = { background: HIGHLIGHT_SELECTED };
      g.moves({ square: selectedSquare, verbose: true }).forEach(m => {
        const isCapture = !!g.get(m.to as Square);
        styles[m.to] = isCapture
          ? { background: 'radial-gradient(circle, rgba(244,67,54,0.55) 84%, transparent 84%)' }
          : { background: `radial-gradient(circle, ${HIGHLIGHT_MOVE} 28%, transparent 28%)` };
      });
    }

    return styles;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fen, selectedSquare, lastMove]);

  // ─── Move list from game ─────────────────────────────────────
  const moveList = gameRef.current.history();

  // ─── Render ──────────────────────────────────────────────────
  return (
    <div style={{ display: 'flex', gap: 24, flexWrap: 'wrap', alignItems: 'flex-start' }}>

      {/* LEFT: board + controls */}
      <div style={{ flex: '0 0 auto' }}>

        {/* Status bar + buttons */}
        <div style={{ marginBottom: 10, display: 'flex', gap: 8, alignItems: 'center' }}>
          <div style={{
            flex: 1, padding: '6px 14px', borderRadius: 6,
            background: '#111', border: '1px solid #222',
            color: status.color, fontFamily: 'monospace', fontSize: 13,
          }}>
            {status.label}
          </div>
          <Btn onClick={undoMove}  disabled={undoStack.current.length === 0 || loading} label="↩ Undo" />
          <Btn onClick={flipBoard} label="⇅ Flip" />
          <Btn onClick={resetGame} label="↺ Reset" />
        </div>

        {/* Board */}
        <div style={{ position: 'relative', width: 440, userSelect: 'none' }}>
          <Chessboard
            options={{
              position: fen,
              onPieceDrop: onPieceDrop,
              onSquareClick: onSquareClick,
              boardOrientation: orientation,
              squareStyles: customSquareStyles,
              darkSquareStyle: { backgroundColor: DARK },
              lightSquareStyle: { backgroundColor: LIGHT },
              allowDragging: !status.over,
              animationDurationInMs: 200
            }}
          />

          {/* Promotion dialog */}
          {pendingPromotion && (
            <div style={{
              position: 'absolute', inset: 0, zIndex: 20,
              background: 'rgba(0,0,0,0.88)',
              display: 'flex', flexDirection: 'column',
              alignItems: 'center', justifyContent: 'center', gap: 16,
              borderRadius: 4,
            }}>
              <div style={{ color: '#C9A84C', fontFamily: 'monospace', fontSize: 13, letterSpacing: 2 }}>
                PROMOTION — CHOOSE PIECE
              </div>
              <div style={{ display: 'flex', gap: 12 }}>
                {(['q','r','b','n'] as PromotionPiece[]).map(p => {
                  const icons: Record<PromotionPiece, string> = { q:'♛',r:'♜',b:'♝',n:'♞' };
                  const names: Record<PromotionPiece, string> = { q:'Queen',r:'Rook',b:'Bishop',n:'Knight' };
                  return (
                    <button
                      key={p}
                      type="button"
                      onClick={() => confirmPromotion(p)}
                      style={{
                        width: 80, height: 80, cursor: 'pointer',
                        background: '#181818', border: '2px solid #C9A84C',
                        borderRadius: 8, color: '#C9A84C',
                        display: 'flex', flexDirection: 'column',
                        alignItems: 'center', justifyContent: 'center', gap: 4,
                      }}
                    >
                      <span style={{ fontSize: 32 }}>{icons[p]}</span>
                      <span style={{ fontSize: 10, fontFamily: 'monospace' }}>{names[p]}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {/* Move history */}
        <div style={{
          marginTop: 10, padding: '8px 12px',
          background: '#0a0a0a', borderRadius: 8, border: '1px solid #1a1a1a',
          fontFamily: 'monospace', fontSize: 12, minHeight: 44, lineHeight: 1.8,
        }}>
          {moveList.length === 0
            ? <span style={{ color: '#333' }}>No moves yet</span>
            : moveList.map((m, i) => (
              <span key={i}>
                {i % 2 === 0 && <span style={{ color: '#444', marginRight: 2 }}>{Math.floor(i/2)+1}.</span>}
                <span style={{ color: i === moveList.length - 1 ? '#C9A84C' : '#666', marginRight: 5 }}>{m}</span>
              </span>
            ))}
        </div>

        {/* FEN */}
        <div style={{
          marginTop: 6, padding: '5px 10px',
          background: '#0a0a0a', borderRadius: 6, border: '1px solid #1a1a1a',
          fontFamily: 'monospace', fontSize: 10, color: '#2a2a2a', wordBreak: 'break-all',
        }}>
          {fen}
        </div>
      </div>

      {/* RIGHT: verdict panel */}
      <div style={{ flex: 1, minWidth: 300 }}>
        {loading && <JudgingSpinner />}
        {!loading && error && <ErrorBanner message={error} />}
        {!loading && !error && verdict && <VerdictDisplay verdict={verdict} />}
        {!loading && !error && !verdict && <EmptyState />}
      </div>
    </div>
  );
}

// ─── Small sub-components ─────────────────────────────────────
function Btn({ onClick, disabled, label }: { onClick: () => void; disabled?: boolean; label: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      style={{
        padding: '6px 12px', borderRadius: 6, cursor: disabled ? 'not-allowed' : 'pointer',
        background: disabled ? '#0d0d0d' : '#1a1a1a',
        color: disabled ? '#2a2a2a' : '#999',
        border: '1px solid #2a2a2a', fontSize: 12, fontFamily: 'monospace',
      }}
    >
      {label}
    </button>
  );
}

function JudgingSpinner() {
  return (
    <div style={{ display:'flex', flexDirection:'column', alignItems:'center', justifyContent:'center', padding: 50, color:'#C9A84C' }}>
      <div style={{ fontSize: 32, marginBottom: 12, display:'inline-block', animation:'spin 1s linear infinite' }}>🌀</div>
      <div style={{ fontFamily:'monospace', letterSpacing: 2 }}>JUDGING MOVE...</div>
      <div style={{ fontSize: 12, color:'#555', marginTop: 8 }}>Dogs evaluating the position...</div>
    </div>
  );
}

function ErrorBanner({ message }: { message: string }) {
  return (
    <div style={{ padding: 16, background:'#F4433611', border:'1px solid #F44336', borderRadius: 8, color:'#F44336', fontSize: 13 }}>
      ⚠ {message}
      <div style={{ fontSize: 11, color:'#F44336aa', marginTop: 6 }}>
        Check the kernel URL in ⚙ Settings (top-right)
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div style={{ display:'flex', flexDirection:'column', alignItems:'center', justifyContent:'center', padding: 60, color:'#3a3a3a', textAlign:'center' }}>
      <div style={{ fontSize: 56, marginBottom: 16 }}>♟</div>
      <div style={{ fontFamily:'monospace', letterSpacing: 2, fontSize: 14, color:'#444' }}>MAKE A MOVE</div>
      <div style={{ fontSize: 12, marginTop: 8, color:'#2a2a2a', lineHeight: 1.7 }}>
        Click a piece then a target square, or drag it.<br />
        Each move is judged by the CYNIC Dogs.
      </div>
    </div>
  );
}
