#!/usr/bin/env python3
"""Runtime principal activation view entrypoint facade.

The concrete view logic is split across focused modules under this package to
keep responsibilities small and respect supervisor organ topology constraints.
"""

from organs.supervisor.src.entrypoints.runtime_principals_views_runtime_reports import *  # noqa: F401,F403
from organs.supervisor.src.entrypoints.runtime_principals_views_organism import *  # noqa: F401,F403
from organs.supervisor.src.entrypoints.runtime_principals_views_surface import *  # noqa: F401,F403
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime import (
    cynic_cli_body,
    launcher_body,
    profile_body,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime_reports import (
    _exists_group,
    _exists_user,
    _owner,
    _run_capture,
    _tail,
)
