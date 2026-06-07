// ChessJudge.tsx — Temporarily disabled for production build.

export interface Props {
  onVerdict?: (verdict: any) => void;
}

export function ChessJudge(_props: Props) {
  return <div style={{ color: '#C9A84C', padding: 20 }}>Chessboard component disabled for debugging.</div>;
}
