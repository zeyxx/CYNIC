"""
CYNIC Dog Auto-Discovery — pkgutil-based registration.

Scans cynic.kernel.organism.brain.cognition.neurons for all AbstractDog subclasses with a DOG_ID class attribute.
Replaces manual import lists — adding a new dog = add the file + DOG_ID.

φ-Law: BURN — discovery replaces boilerplate.
"""
from __future__ import annotations

import importlib
import logging
import pkgutil
from typing import Any

import cynic.kernel.organism.brain.cognition.neurons as dogs_pkg
from cynic.kernel.organism.brain.cognition.neurons.base import AbstractDog, DogId

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.neurons.discovery")


def discover_dog_classes() -> dict[str, type[AbstractDog]]:
    """
    Scan cynic.kernel.organism.brain.cognition.neurons for all concrete AbstractDog subclasses with DOG_ID.

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
            module = importlib.import_module(f"cynic.kernel.organism.brain.cognition.neurons.{module_name}")
        except Exception:
            logger.warning("Failed to import cynic.kernel.organism.brain.cognition.neurons.%s", module_name, exc_info=True)
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
    
    New Path: Uses MasterDog + registry for all 11 Dogs.
    Legacy Path: Still checks files for custom Dog classes (overrides).
    """
    from cynic.kernel.organism.brain.cognition.neurons.master import MasterDog
    from cynic.kernel.organism.brain.cognition.neurons.registry import get_soul

    # 1. Start with the 11 standard Dogs from registry
    dogs: dict[str, AbstractDog] = {}
    for dog_id in DogId:
        soul = get_soul(dog_id)
        dogs[dog_id.value] = MasterDog(soul)

    # 2. Apply explicit overrides (passed as args)
    for dog_id_val, instance in overrides.items():
        dogs[dog_id_val] = instance

    # 3. Discovery fallback: If a custom class exists in a file, it wins over MasterDog
    # This ensures backward compatibility during migration
    classes = discover_dog_classes()
    for dog_id_val, cls in classes.items():
        if dog_id_val not in overrides:
            # Check if it's NOT a MasterDog (real custom class)
            if "MasterDog" not in cls.__name__:
                logger.info(f"Discovery: Custom class {cls.__name__} found for {dog_id_val}, overriding MasterDog.")
                dogs[dog_id_val] = cls()

    return dogs
