"""Dynamic registry.

Harnesses are discovered, never hardcoded. Adding a harness means registering an
adapter, not editing a central switch. This is the project rule: dynamic registries
over hardcoded hosts, paths, and names.
"""

from __future__ import annotations

from typing import Callable, Generic, Iterator, TypeVar

from .errors import UnknownHarness

T = TypeVar("T")


class Registry(Generic[T]):
    """A minimal name -> implementation registry."""

    def __init__(self, kind: str) -> None:
        self._kind = kind
        self._items: dict[str, T] = {}

    def register(self, name: str) -> Callable[[T], T]:
        def decorator(impl: T) -> T:
            if name in self._items:
                raise ValueError(f"{self._kind} '{name}' is already registered")
            self._items[name] = impl
            return impl

        return decorator

    def get(self, name: str) -> T:
        try:
            return self._items[name]
        except KeyError as exc:
            raise UnknownHarness(
                f"no {self._kind} registered for '{name}'; known: {sorted(self._items)}"
            ) from exc

    def __contains__(self, name: object) -> bool:
        return name in self._items

    def __iter__(self) -> Iterator[str]:
        return iter(sorted(self._items))

    def names(self) -> list[str]:
        return sorted(self._items)
