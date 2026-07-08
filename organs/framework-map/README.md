# organ-framework-map

Maps external agent frameworks into CYNIC contracts.

This organ observes frameworks such as Hermes Agent from a local checkout, then
emits JSON reports for capabilities, dependencies, architecture patterns and
CYNIC compatibility. It does not run the framework as the organism center and
does not vendor upstream source into CYNIC.

Default Hermes source path:

```bash
external/hermes-agent/
```

That path is ignored by Git. Clone or mirror upstream there locally, then run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 organs/framework-map/src/entrypoints/cli.py map-hermes
```
