# CYNIC Public Edge

Contract-first scaffold for making CYNIC legible from the outside.

The goal is not to ship a dashboard first. The goal is to define what the
interface must transmit before implementing the product surface.

The source contract is the root axiom:

- `axioms/one-person-enterprise-interface-v1.json`

It defines:

- `identity`
- `operating_model`
- `interface_model`
- `archetype_model`
- `axiom_model`
- `kenosis_protocol`
- `audience_model`
- `design_policy`
- `projection_rules`

The React app imports that axiom directly and renders it as a design surface. This keeps the public
edge honest while the real proof objects, Solana transaction references, and
corporate boundaries are still being designed.

## Development

```bash
npm install
npm run dev
npm run build
npm run lint
```
