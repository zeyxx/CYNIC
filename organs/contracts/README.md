# organ-contracts

Shared surface contracts for command and report manifests.

This organ owns the universal interfaces used by multiple organs:

- `CommandSurfaceSpec`
- `ReportSurfaceSpec`
- `SurfaceRegistry`
- `attach_surface_contract`

It must remain free of organ-specific business logic.
