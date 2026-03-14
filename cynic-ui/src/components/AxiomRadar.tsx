import {
  RadarChart, PolarGrid, PolarAngleAxis,
  Radar, ResponsiveContainer, Tooltip,
} from 'recharts';
import type { QScore } from '../types';
import { AXIOMS, PHI_MAX } from '../types';

interface Props {
  qScore: QScore;
  color: string;
}

export function AxiomRadar({ qScore, color }: Props) {
  const data = AXIOMS.map(({ key, label }) => ({
    axiom: label,
    value: Math.round((qScore[key] / PHI_MAX) * 100),
    raw: qScore[key].toFixed(3),
  }));

  return (
    <ResponsiveContainer width="100%" height={240}>
      <RadarChart data={data} margin={{ top: 10, right: 20, bottom: 10, left: 20 }}>
        <PolarGrid stroke="rgba(255,255,255,0.08)" />
        <PolarAngleAxis
          dataKey="axiom"
          tick={{ fill: '#888', fontSize: 11, fontFamily: 'monospace' }}
        />
        <Radar
          name="axioms"
          dataKey="value"
          stroke={color}
          fill={color}
          fillOpacity={0.15}
          strokeWidth={1.5}
        />
        <Tooltip
          formatter={(_value, name, props) => [
            `${(props.payload as { raw: string }).raw} / ${PHI_MAX.toFixed(3)}`,
            String(name),
          ]}
          contentStyle={{
            background: '#111', border: `1px solid ${color}`,
            borderRadius: '6px', fontSize: '12px', color: '#e0e0e0',
          }}
        />
      </RadarChart>
    </ResponsiveContainer>
  );
}
