import {
  RadarChart,
  PolarGrid,
  PolarAngleAxis,
  Radar,
  ResponsiveContainer,
  Tooltip,
} from 'recharts';
import { AXIOM_COLORS, AXIOM_ICONS } from '../types';
import type { QScore } from '../types';

interface Props {
  qScore: QScore;
}

const AXIOMS = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'sovereignty'] as const;

export function AxiomChart({ qScore }: Props) {
  const data = AXIOMS.map((key) => ({
    axiom: `${AXIOM_ICONS[key]} ${key.toUpperCase()}`,
    score: Math.round(qScore[key] * 1000) / 1000,
    fullMark: 0.618,
  }));

  return (
    <div style={{ width: '100%', height: 280 }}>
      <ResponsiveContainer>
        <RadarChart data={data}>
          <PolarGrid stroke="#333" />
          <PolarAngleAxis
            dataKey="axiom"
            tick={{ fill: '#aaa', fontSize: 11 }}
          />
          <Radar
            name="Score"
            dataKey="score"
            stroke="#C9A84C"
            fill="#C9A84C"
            fillOpacity={0.35}
          />
          <Tooltip
            contentStyle={{ background: '#1a1a1a', border: '1px solid #333', borderRadius: 6 }}
            formatter={(v: any) => [Number(v).toFixed(3), 'Score']}
          />
        </RadarChart>
      </ResponsiveContainer>
    </div>
  );
}

interface BarsProps {
  qScore: QScore;
}

export function AxiomBars({ qScore }: BarsProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {AXIOMS.map((key) => {
        const value = qScore[key];
        const pct = (value / 0.618) * 100;
        const color = AXIOM_COLORS[key];
        return (
          <div key={key}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
              <span style={{ fontSize: 12, color: '#aaa' }}>
                {AXIOM_ICONS[key]} {key.toUpperCase()}
              </span>
              <span style={{ fontSize: 12, color, fontFamily: 'monospace' }}>
                {value.toFixed(3)}
              </span>
            </div>
            <div style={{ height: 6, background: '#2a2a2a', borderRadius: 3 }}>
              <div style={{
                height: '100%',
                width: `${pct}%`,
                background: color,
                borderRadius: 3,
                transition: 'width 0.6s ease',
              }} />
            </div>
          </div>
        );
      })}
    </div>
  );
}
