import type { ReactNode } from 'react';

interface SurfacePanelProps {
  eyebrow?: string;
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}

export function SurfacePanel({ eyebrow, title, subtitle, actions, children, className = '' }: SurfacePanelProps) {
  return (
    <section className={`surface-panel ${className}`.trim()}>
      <div className="surface-panel-head">
        <div className="surface-panel-copy">
          {eyebrow ? <div className="section-label">{eyebrow}</div> : null}
          <h2 className="section-title">{title}</h2>
          {subtitle ? <p className="section-note">{subtitle}</p> : null}
        </div>
        {actions ? <div className="surface-panel-actions">{actions}</div> : null}
      </div>
      <div className="surface-panel-body">{children}</div>
    </section>
  );
}
