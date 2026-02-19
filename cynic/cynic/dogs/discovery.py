"""
CYNIC Dog Auto-Discovery — pkgutil-based registration.

Scans cynic.dogs for all AbstractDog subclasses with a DOG_ID class attribute.
Replaces manual import lists — adding a new dog = add the file + DOG_ID.

φ-Law: BURN — discovery replaces boilerplate.
"""
from __future__ import annotations

import importlib
import logging
import pkgutil
from typing import Any

import cynic.dogs as dogs_pkg
from cynic.dogs.base import AbstractDog, DogId

logger = logging.getLogger("cynic.dogs.discovery")


def discover_dog_classes() -> dict[str, type[AbstractDog]]:
    """
    Scan cynic.dogs for all concrete AbstractDog subclasses with DOG_ID.

    Returns:
        dict mapping DogId value → Dog class (NOT instances).

    Raises:
        ValueError: if a Dog class has a DOG_ID not in DogId enum,
                    or if two classes share the same DOG_ID.
    """
    found: dict[str, type[AbstractDog]] = {}
    valid_ids = set(DogId)

    for _, module_name, _ in pkgutil.iter_modules(dogs_pkg.__path__):
        if module_name.startswith("_") or module_name in ("base", "discovery"):
            continue

        try:
            module = importlib.import_module(f"cynic.dogs.{module_name}")
        except Exception:
            logger.warning("Failed to import cynic.dogs.%s", module_name, exc_info=True)
            continue

        for attr_name in dir(module):
            attr = getattr(module, attr_name)
            if (
                isinstance(attr, type)
                and issubclass(attr, AbstractDog)
                and attr is not AbstractDog
                and not getattr(attr, "__abstractmethods__", None)
                and hasattr(attr, "DOG_ID")
            ):
                dog_id = attr.DOG_ID
                if dog_id not in valid_ids:
                    raise ValueError(
                        f"{attr.__name__}.DOG_ID = {dog_id!r} is not a valid DogId"
                    )
                if dog_id in found:
                    raise ValueError(
                        f"Duplicate DOG_ID {dog_id}: "
                        f"{found[dog_id].__name__} and {attr.__name__}"
                    )
                found[dog_id] = attr

    logger.info("Discovered %d dog classes: %s", len(found), sorted(found.keys()))
    return found


def discover_dogs(**overrides: Any) -> dict[str, AbstractDog]:
    """
    Discover and instantiate all dogs.

    Most dogs are instantiated with no args. Override specific dogs
    by passing keyword arguments keyed by DogId value:

        dogs = discover_dogs(ORACLE=OracleDog(qtable=my_qtable))

    Returns:
        dict mapping DogId value → Dog instance.
    """
    classes = discover_dog_classes()
    dogs: dict[str, AbstractDog] = {}

    for dog_id, cls in classes.items():
        if dog_id in overrides:
            dogs[dog_id] = overrides[dog_id]
        else:
            try:
                dogs[dog_id] = cls()
            except TypeError as exc:
                logger.warning(
                    "Cannot auto-instantiate %s (needs args): %s. "
                    "Pass it via overrides.",
                    cls.__name__,
                    exc,
                )

    return dogs
