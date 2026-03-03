"""
CYNIC Dog Auto-Discovery â€" pkgutil-based registration.

Scans cynic.kernel.organism.brain.cognition.neurons for all AbstractDog subclasses with a DOG_ID class attribute.
Replaces manual import lists â€" adding a new dog = add the file + DOG_ID.

Ï-Law: BURN â€" discovery replaces boilerplate.
"""

from __future__ import annotations

import importlib
import logging
import pkgutil
from typing import Any

import cynic.kernel.organism.brain.cognition.neurons as dogs_pkg
from cynic.kernel.organism.brain.cognition.neurons.base import AbstractDog, DogId

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.neurons.discovery")


import pathlib

def discover_dog_classes() -> dict[str, type[AbstractDog]]:
    """
    Scan cynic.kernel.organism.brain.cognition.neurons for all concrete AbstractDog subclasses with DOG_ID.
    """
    found: dict[str, type[AbstractDog]] = {}
    valid_ids = set(DogId)
    
    # Noble Path Discovery (Robust on Windows/Linux)
    pkg_dir = pathlib.Path(__file__).parent
    
    for py_file in pkg_dir.glob("*.py"):
        module_name = py_file.stem
        if module_name.startswith("_") or module_name in ("base", "discovery"):
            continue

        try:
            module = importlib.import_module(
                f"cynic.kernel.organism.brain.cognition.neurons.{module_name}"
            )
        except Exception as e:
            logger.warning(f"Failed to import neuron {module_name}: {e}")
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
                    raise ValueError(f"{attr.__name__}.DOG_ID = {dog_id!r} is not a valid DogId")
                if dog_id in found:
                    raise ValueError(
                        f"Duplicate DOG_ID {dog_id}: "
                        f"{found[dog_id].__name__} and {attr.__name__}"
                    )
                found[dog_id] = attr

    logger.info("Discovered %d dog classes: %s", len(found), sorted(found.keys()))
    return found


def discover_dogs(bus: Any, llm_registry: Any | None = None, vascular: Any | None = None, **overrides: Any) -> dict[str, AbstractDog]:
    """
    Discover and instantiate all dogs.
    """
    from cynic.kernel.organism.brain.cognition.neurons.master import MasterDog
    from cynic.kernel.organism.brain.cognition.neurons.registry import get_soul

    # 1. Start with the 11 standard Dogs from registry
    dogs: dict[str, AbstractDog] = {}
    for dog_id in DogId:
        soul = get_soul(dog_id)
        dog = MasterDog(soul, bus=bus, vascular=vascular)
        if llm_registry:
            dog.set_llm_registry(llm_registry)
        dogs[dog_id.value] = dog

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
                logger.info(
                    f"Discovery: Custom class {cls.__name__} found for {dog_id_val}, overriding MasterDog."
                )
                dogs[dog_id_val] = cls()

    return dogs
