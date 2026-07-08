import contract from '../../../axioms/one-person-enterprise-interface-v1.json';
import './index.css';

type Audience = {
  name: string;
  need: string;
  primary_emotion: string;
};

type EmotionalTarget = {
  emotion: string;
  interface_signal: string;
};

type Rail = {
  name: string;
  owns: string;
  source_of_truth: string;
  interface_emotion: string;
};

type Route = {
  path: string;
  purpose: string;
};

type Archetype = {
  name: string;
  organism_function: string;
  current_expression: string;
  failure_mode: string;
};

type ArchetypeBalance = {
  stage: string;
  required_mix: string[];
  observed_strengths: string[];
  observed_gaps: string[];
  interface_priority: string;
};

type ActiveAxiom = {
  name: string;
  product_question: string;
  interface_obligation: string;
};

type ObservedAxiom = {
  name: string;
  status: string;
  interface_obligation: string;
};

type ArchetypeAxiom = {
  archetype: string;
  primary_axioms: string[];
  must_prove: string;
};

type AxiomMap = {
  active_axioms: ActiveAxiom[];
  observed_axioms: ObservedAxiom[];
  archetype_axiom_matrix: ArchetypeAxiom[];
};

function App() {
  const identity = contract.identity;
  const audiences = contract.audience_model.audiences as Audience[];
  const emotions = contract.design_policy.emotional_targets as EmotionalTarget[];
  const rails = contract.operating_model.rails as Rail[];
  const routes = contract.interface_model.first_routes as Route[];
  const archetypes = contract.archetype_model.archetypes as Archetype[];
  const balance = contract.archetype_model.current_balance as ArchetypeBalance;
  const axiomMap = contract.axiom_model as AxiomMap;

  return (
    <main className="design-shell">
      <section className="hero">
        <p className="eyebrow">Interface contract before interface</p>
        <h1>{identity.surface}</h1>
        <p>{identity.purpose}</p>
        <code>{identity.source_framework}</code>
      </section>

      <section className="section boundary-section">
        <div className="section-intro">
          <p className="eyebrow">Canonical loop</p>
          <h2>Turn private work into public decisions.</h2>
        </div>

        <ol className="loop-strip">
          {contract.operating_model.canonical_loop.map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ol>
      </section>

      <section className="section boundary-section">
        <div className="section-intro">
          <p className="eyebrow">Five rails</p>
          <h2>A claim must know which rail owns it.</h2>
        </div>

        <div className="boundary-grid">
          {rails.map((rail) => (
            <article className="boundary-card" key={rail.name}>
              <span>{rail.name}</span>
              <h3>{rail.owns}</h3>
              <p>{rail.source_of_truth}</p>
              <strong>{rail.interface_emotion}</strong>
            </article>
          ))}
        </div>
      </section>

      <section className="section">
        <div className="section-intro">
          <p className="eyebrow">First product routes</p>
          <h2>Start with manifest, proof, decisions, outcomes.</h2>
        </div>

        <div className="route-grid">
          {routes.map((route) => (
            <article className="route-card" key={route.path}>
              <strong>{route.path}</strong>
              <p>{route.purpose}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="section">
        <div className="section-intro">
          <p className="eyebrow">Organism archetypes</p>
          <h2>Use roles as metabolic modes, not job titles.</h2>
        </div>

        <div className="balance-panel">
          <div>
            <span>Stage</span>
            <strong>{balance.stage}</strong>
          </div>
          <div>
            <span>Required mix</span>
            <strong>{balance.required_mix.join(' + ')}</strong>
          </div>
          <div>
            <span>Current gap</span>
            <strong>{balance.observed_gaps.join(' + ')}</strong>
          </div>
          <p>{balance.interface_priority}</p>
        </div>

        <div className="archetype-grid">
          {archetypes.map((archetype) => (
            <article className="archetype-card" key={archetype.name}>
              <h3>{archetype.name}</h3>
              <p>{archetype.organism_function}</p>
              <dl>
                <div>
                  <dt>Expression</dt>
                  <dd>{archetype.current_expression}</dd>
                </div>
                <div>
                  <dt>Failure</dt>
                  <dd>{archetype.failure_mode}</dd>
                </div>
              </dl>
            </article>
          ))}
        </div>
      </section>

      <section className="section">
        <div className="section-intro">
          <p className="eyebrow">Axiom map</p>
          <h2>Translate work modes into CYNIC principles.</h2>
        </div>

        <div className="axiom-grid">
          {axiomMap.active_axioms.map((axiom) => (
            <article className="axiom-card" key={axiom.name}>
              <strong>{axiom.name}</strong>
              <p>{axiom.product_question}</p>
              <span>{axiom.interface_obligation}</span>
            </article>
          ))}
        </div>

        <div className="observed-axioms">
          {axiomMap.observed_axioms.map((axiom) => (
            <article key={axiom.name}>
              <strong>{axiom.name}</strong>
              <span>{axiom.status}</span>
              <p>{axiom.interface_obligation}</p>
            </article>
          ))}
        </div>

        <div className="matrix-table" aria-label="Archetype to axiom matrix">
          {axiomMap.archetype_axiom_matrix.map((row) => (
            <article key={row.archetype}>
              <strong>{row.archetype}</strong>
              <span>{row.primary_axioms.join(' + ')}</span>
              <p>{row.must_prove}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="section split">
        <div>
          <p className="eyebrow">What we want</p>
          <ul className="statement-list">
            {contract.design_policy.what_we_want.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>

        <div>
          <p className="eyebrow">What we must do</p>
          <ul className="statement-list strong">
            {contract.design_policy.what_we_must_do.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>
      </section>

      <section className="section">
        <div className="section-intro">
          <p className="eyebrow">Audience contracts</p>
          <h2>Each audience must feel something precise.</h2>
        </div>

        <div className="audience-table">
          {audiences.map((audience) => (
            <article className="audience-row" key={audience.name}>
              <strong>{audience.name}</strong>
              <p>{audience.need}</p>
              <span>{audience.primary_emotion}</span>
            </article>
          ))}
        </div>
      </section>

      <section className="section emotion-section">
        <div className="section-intro">
          <p className="eyebrow">Emotional design</p>
          <h2>The interface should transmit discipline, not mythology.</h2>
        </div>

        <div className="emotion-grid">
          {emotions.map((target) => (
            <article className="emotion-card" key={target.emotion}>
              <h3>{target.emotion}</h3>
              <p>{target.interface_signal}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="section split refusal-section">
        <div>
          <p className="eyebrow">Refusals</p>
          <ul className="statement-list danger">
            {contract.design_policy.what_we_refuse.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>

        <div>
          <p className="eyebrow">First scaffold</p>
          <ol className="ordered-list">
            {contract.interface_model.first_scaffold.sections.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ol>
        </div>
      </section>
    </main>
  );
}

export default App;
