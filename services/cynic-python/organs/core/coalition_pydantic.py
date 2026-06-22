"""
Option A — Coalition via Pydantic + instructor.

Transforme la frontière de coalition en validateurs Pydantic réutilisables.
Quand un agent génère un payload via instructor, les règles de coalition
sont appliquées au niveau du schéma Pydantic : si le payload sort des clous,
le validateur Pydantic lève une ValidationError avant même que le payload
n'atteigne l'organe effecteur.

Flot:
  1. Agent génère un payload structuré (JSON)
  2. instructor hydrate le modèle Pydantic cible
  3. Les validateurs Pydantic (field_validator, model_validator) vérifient
     les contraintes de coalition
  4. Si violation -> ValidationError -> instructor force l'agent à re-générer
     avec le message d'erreur dans le prompt
  5. L'agent reçoit format_violation_for_instructor() dans son contexte
     et doit produire une réponse conforme
"""

import logging
from typing import Any, ClassVar, Optional

from pydantic import BaseModel, model_validator

from .coalition import (
    CoalitionBoundary,
    CoalitionVariable,
    format_violation_for_instructor,
)

logger = logging.getLogger("coalition_pydantic")


class CoalitionBaseModel(BaseModel):
    """BaseModel Pydantic lié à une frontière de coalition.

    Usage:
        class TokenConfig(CoalitionBaseModel):
            _coalition: ClassVar[CoalitionBoundary] = my_coalition
            max_tokens: int
    """

    _coalition: ClassVar[Optional[CoalitionBoundary]] = None

    @model_validator(mode="wrap")
    @classmethod
    def _coalition_validator_wrapper(cls, data: Any, handler: Any) -> Any:
        """Wrap le validateur standard Pydantic pour ajouter la validation coalition."""
        result = handler(data)

        coalition = getattr(cls, "_coalition", None)
        if coalition is None:
            return result

        if not isinstance(data, dict):
            return result

        errors = []
        for field_name, value in data.items():
            error = coalition.validate(field_name, value)
            if error is not None:
                errors.append(error)

        if errors:
            error_msg = "; ".join(errors)
            coalition_msg = format_violation_for_instructor(error_msg)
            raise ValueError(coalition_msg)

        return result

    @model_validator(mode="after")
    def _coalition_after_validator(self) -> "CoalitionBaseModel":
        """Validateur post-construction pour les règles cross-champ."""
        coalition = getattr(self.__class__, "_coalition", None)
        if coalition is None:
            return self

        # Use class level model_fields to avoid Pydantic V2.11 deprecation
        view = coalition.get_view()
        for field_name in self.__class__.model_fields:
            variable = coalition._variables.get(field_name)
            if variable is None:
                continue
            value = getattr(self, field_name, None)
            if value is None:
                continue
            error = variable.validate(view, value)
            if error is not None:
                coalition_msg = format_violation_for_instructor(
                    f"[COALITION:{coalition.name}] Violation cross-champ "
                    f"sur '{field_name}': {error}"
                )
                raise ValueError(coalition_msg)

        return self


# ──────────────────────────────────────────────────────────────
# Utilitaires pour créer des liants Pydantic-coalition dynamiques
# ──────────────────────────────────────────────────────────────

def coalition_field(
    variable_name: str,
    pydantic_field_type: type,
    pydantic_description: str = "",
    **pydantic_kwargs: Any,
) -> tuple:
    """Crée un champ Pydantic lié à une variable de coalition."""
    from pydantic import Field

    field_info = Field(
        description=pydantic_description,
        **pydantic_kwargs,
    )
    return (pydantic_field_type, field_info)


# ──────────────────────────────────────────────────────────────
# Injectable validator pour instructor
# ──────────────────────────────────────────────────────────────

def make_coalition_validator(
    coalition: CoalitionBoundary,
) -> Any:
    """Crée un validateur callable pour instructor qui vérifie la coalition.

    Usage avec instructor:
        validator = make_coalition_validator(coalition)

        config = client.chat.completions.create(
            model="claude-3",
            response_model=TradeConfig,
            validation_context={"coalition_validator": validator},
            messages=[...],
        )
    """

    def validator(model: BaseModel) -> BaseModel:
        """Valide un modèle Pydantic contre la coalition."""
        for field_name in model.__class__.model_fields:
            value = getattr(model, field_name, None)
            if value is None:
                continue
            error = coalition.validate(field_name, value)
            if error is not None:
                coalition_msg = format_violation_for_instructor(error)
                raise ValueError(coalition_msg)
        return model

    return validator