# Organ: Jupyter (organ-jupyter)

## 1. Identity & Role
- **Name**: `organ-jupyter`
- **Instance**: `organ-jupyter-hermes-agent`
- **Role**: Notebook Surface Manager / Data Science Environment
- **Scope**: Local + JupyterLab

## 2. Purpose
`organ-jupyter` serves as the exploratory data analysis (EDA), machine learning experimentation, and interactive coding environment for the CYNIC ecosystem. It provides a stateful execution context where agents and humans can run notebooks (`.ipynb`), visualize data, and test models before integrating them into the core infrastructure.

## 3. Sensors (Perception)
- Listens to workspace events for `.ipynb` modifications.
- Monitors kernel execution states (Idle, Busy, Error).
- Tracks memory and GPU usage allocated to Jupyter execution environments.

## 4. Reactions (Transformation & Structuration)
- Automatically manages Jupyter execution environments (starting, stopping kernels).
- Periodically checkpoints execution outputs into persistent storage.
- Synchronizes output data artifacts generated during exploration to `data/` directories if flagged for persistence.

## 5. Security & Isolation
- The Jupyter environment runs locally but is isolated from direct public internet exposure unless proxied through Tailscale.
- Output serialization sanitizes secrets before committing `.ipynb` files to Git.

## 6. Lifecycle
- Supervised by the Hermes Agent runtime.
- Evaluated via `.gate-0` hooks to strip unnecessary cell outputs if they exceed size limits before git pushes.
