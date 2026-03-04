"""
CYNIC Heresy Filter - The Syntax Firewall.
Respects Security & AI Infra Lenses.

Prevents structurally invalid code (e.g., LLM hallucinations) from ever being
written to the physical disk. Uses native AST parsing as the ultimate judge.
"""

from __future__ import annotations

import ast
import logging
from typing import Tuple

logger = logging.getLogger("cynic.security.heresy_filter")


class HeresyFilter:
    """
    The gatekeeper between LLM thoughts and disk reality.
    """

    @staticmethod
    def is_clean(code_content: str) -> Tuple[bool, str]:
        """
        Parses the code in memory.
        Returns (True, "OK") if clean, or (False, ErrorMessage) if it contains a heresy.
        """
        try:
            ast.parse(code_content)
            return True, "OK"
        except SyntaxError as e:
            # We construct a highly descriptive error string to feed back to the LLM
            error_msg = f"SyntaxError at line {e.lineno}, offset {e.offset}: {e.msg}\n"
            if e.text:
                error_msg += f"Problematic code: {e.text.strip()}"

            logger.warning(f"Heresy blocked before disk write: {error_msg}")
            return False, error_msg
        except Exception as e:
            logger.error(f"Unexpected parsing error: {e}")
            return False, str(e)
