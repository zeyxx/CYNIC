"""CYNIC metabolism package — cost tracking for all LLM/API calls."""
from __future__ import annotations

import sys
import types


class _MetabolismModule(types.ModuleType):
    """Custom module class that keeps submodule references alive in sys.modules.

    When tests delete 'metabolism.cost_tracker' from sys.modules and then
    do 'from metabolism import cost_tracker', Python returns the cached
    attribute but doesn't re-add it to sys.modules — which breaks
    importlib.reload(). This custom __getattribute__ re-registers any
    submodule attribute on every access, keeping sys.modules consistent.
    """

    def __getattribute__(self, name: str) -> object:
        val = super().__getattribute__(name)
        if isinstance(val, types.ModuleType) and val.__name__ != self.__name__:
            sys.modules.setdefault(val.__name__, val)
            # Always re-register so reload() can find it even after deletion
            sys.modules[val.__name__] = val
        return val


sys.modules[__name__].__class__ = _MetabolismModule
