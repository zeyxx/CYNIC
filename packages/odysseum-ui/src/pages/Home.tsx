import { useMemo, useState } from 'react';

import '../index.css';

type RailId = 'vault' | 'governance' | 'ignis' | 'gasdf';

const rails: Array<{
  id: RailId;
  label: string;
  status: string;
  metric: string;
  summary: string;
}> = [
  {
    id: 'vault',
    label: 'Futarchy Vault',
    status: 'Spec locked',
    metric: '0.95% fee route',
    summary: 'Routes dynamic fees into a non-custodial treasury controlled by conditional markets.',
  },
  {
    id: 'governance',
    label: 'Options on Solana',
    status: 'Market design',
    metric: 'Pass / Fail',
    summary: 'Turns proposals into tradable governance derivatives before any treasury action executes.',
  },
  {
    id: 'ignis',
    label: 'Ignis Engine',
    status: 'Program draft',
    metric: 'Buyback burn',
    summary: 'Uses collected fees for mathematically bounded buyback and burn without discretionary payouts.',
  },
  {
    id: 'gasdf',
    label: 'GASdf Relay',
    status: 'Boundary active',
    metric: 'Gasless route',
    summary: 'Separates human operators from transaction execution through a meta-signer relay boundary.',
  },
];

const communicationRows = [
  ['Operator', 'Telegram / Signal', 'Escalation, approvals, launch windows'],
  ['Market', 'X / Superteam', 'Build in public, expose problems, gather Solana-native feedback'],
  ['Builders', 'GitHub / Docs / Colosseum', 'SDK, integration specs, audits, hackathon shipping'],
  ['Enterprise', 'TALARIA SAS', 'Contracts, invoices, partnership intake'],
];

const lifecycleSteps = [
  ['Raw token', 'No durable fee route; human capture risk remains high.'],
  ['Candidate', 'Token submits mint, liquidity, fee and community evidence.'],
  ['Odysseum-Locked', 'Vault route and governance controls are verified.'],
  ['Conglomerate', 'Franchise, acquisition or parent-community routing is live.'],
];

function formatCurrency(value: number) {
  return new Intl.NumberFormat('en-US', {
    maximumFractionDigits: 0,
    style: 'currency',
    currency: 'USD',
  }).format(value);
}

export default function Home() {
  const [activeRail, setActiveRail] = useState<RailId>('vault');
  const [dailyVolume, setDailyVolume] = useState(850_000);
  const [takeRateBps, setTakeRateBps] = useState(95);

  const selectedRail = rails.find((rail) => rail.id === activeRail) ?? rails[0];
  const dailyCapture = useMemo(
    () => dailyVolume * (takeRateBps / 10_000),
    [dailyVolume, takeRateBps],
  );

  return (
    <>


      <main>
        <section className="command-surface" id="command">
          <img className="command-media" src="/hero-bg.jpg" alt="Odysseum vault interface" />
          <div className="command-copy">
            <p className="eyebrow">Futarchic OS for Solana communities</p>
            <h1>Odysseum Control Surface</h1>
            <p className="lead">
              A working interface for turning raw community tokens into Odysseum-Locked assets:
              fee routing, conditional governance, burn execution and public communication rails.
            </p>
            <div className="actions">
              <a className="button primary" href="#standard">Open Standard</a>
              <a className="button secondary" href="#communications">Route Comms</a>
            </div>
          </div>

          <aside className="status-board" aria-label="Odysseum status board">
            <div className="board-row">
              <span>Manifest</span>
              <strong>Normative</strong>
            </div>
            <div className="board-row">
              <span>Public domain</span>
              <strong>odysseum.talaria.build</strong>
            </div>
            <div className="board-row">
              <span>Primary package</span>
              <strong>packages/odysseum-ui</strong>
            </div>
            <div className="board-row alert">
              <span>Launch pressure</span>
              <strong>High</strong>
            </div>
          </aside>
        </section>

        <section className="section" id="standard">
          <div className="section-heading">
            <p className="eyebrow">Odysseum-Locked standard</p>
            <h2>From raw token to governed asset</h2>
          </div>

          <div className="rail-tabs" role="tablist" aria-label="Odysseum subsystem selector">
            {rails.map((rail) => (
              <button
                className={rail.id === activeRail ? 'tab active' : 'tab'}
                key={rail.id}
                onClick={() => setActiveRail(rail.id)}
                type="button"
              >
                {rail.label}
              </button>
            ))}
          </div>

          <div className="standard-grid">
            <article className="rail-detail">
              <p className="rail-status">{selectedRail.status}</p>
              <h3>{selectedRail.label}</h3>
              <p>{selectedRail.summary}</p>
              <div className="metric-strip">
                <span>Primary metric</span>
                <strong>{selectedRail.metric}</strong>
              </div>
            </article>

            <article className="simulator">
              <h3>Vault Capture Simulator</h3>
              <label htmlFor="daily-volume">Daily token volume</label>
              <input
                id="daily-volume"
                max="5000000"
                min="10000"
                onChange={(event) => setDailyVolume(Number(event.target.value))}
                step="10000"
                type="range"
                value={dailyVolume}
              />
              <div className="input-line">
                <span>{formatCurrency(dailyVolume)}</span>
                <input
                  aria-label="Take rate basis points"
                  max="300"
                  min="10"
                  onChange={(event) => setTakeRateBps(Number(event.target.value))}
                  type="number"
                  value={takeRateBps}
                />
                <span>bps</span>
              </div>
              <div className="capture-total">
                <span>Estimated daily vault capture</span>
                <strong>{formatCurrency(dailyCapture)}</strong>
              </div>
            </article>
          </div>

          <div className="lifecycle-list" aria-label="Token lifecycle">
            {lifecycleSteps.map(([title, description], index) => (
              <article className="step-card" key={title}>
                <span>{String(index + 1).padStart(2, '0')}</span>
                <h3>{title}</h3>
                <p>{description}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="section split" id="communications">
          <div>
            <p className="eyebrow">Communication rails</p>
            <h2>One protocol, four audiences</h2>
            <p className="section-copy">
              Odysseum needs communication surfaces that do not mix legal intake, market narrative,
              operator approvals and builder documentation. Each rail has a different risk profile.
            </p>
          </div>
          <div className="comms-table" role="table" aria-label="Communication rail map">
            {communicationRows.map(([audience, channel, purpose]) => (
              <div className="comms-row" role="row" key={audience}>
                <strong>{audience}</strong>
                <span>{channel}</span>
                <p>{purpose}</p>
              </div>
            ))}
          </div>
        </section>

        <section className="section sas-band" id="sas">
          <div className="section-heading">
            <p className="eyebrow">TALARIA SAS interface</p>
            <h2>Business boundary without protocol custody</h2>
          </div>
          <div className="sas-grid">
            <article>
              <h3>Owns</h3>
              <p>Enterprise intake, partnership agreements, invoices, commercial support and public deployment operations.</p>
            </article>
            <article>
              <h3>Does not own</h3>
              <p>Vault custody, governance outcomes, market resolution or discretionary treasury execution.</p>
            </article>
            <article>
              <h3>Next interface</h3>
              <p>Founder/operator intake form, token readiness checklist and public Odysseum-Locked verification view.</p>
            </article>
          </div>
        </section>
      </main>


    </>
  );
}
