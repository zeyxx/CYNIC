from __future__ import annotations
import logging, time
from typing import Any, Optional
from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.cognition.cortex.handlers.registry import HandlerRegistry
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import JudgmentPipeline
logger = logging.getLogger("cynic.cognition.cortex.handlers.composer")
class HandlerError(Exception):
 def __init__(self, handler_id: str, error: str) -> None:
  self.handler_id = handler_id
  self.error = error
  super().__init__(f"Handler '{handler_id}' failed: {error}")
class HandlerComposer:
 def __init__(self, registry: HandlerRegistry) -> None:
  self.registry = registry
 async def compose(self, pipeline: JudgmentPipeline, level: Optional[ConsciousnessLevel]=None, budget_usd: float=0.0) -> HandlerResult:
  t0 = time.perf_counter()
  handlers_executed: list[str] = []
  try:
   level_selector = self.registry.get("level_selector")
   level_result = await level_selector.execute(pipeline=pipeline, cell=pipeline.cell, budget_usd=budget_usd, requested_level=level)
   if not level_result.success:
    raise HandlerError("level_selector", level_result.error)
   selected_level: ConsciousnessLevel = level_result.output
   pipeline.level = selected_level
   handlers_executed.append("level_selector")
   cycle_handler_map = {"REFLEX":"cycle_reflex","MICRO":"cycle_micro","MACRO":"cycle_macro"}
   cycle_handler_id = cycle_handler_map.get(selected_level.name)
   if not cycle_handler_id:
    raise ValueError(f"Unknown level: {selected_level}")
   cycle_handler = self.registry.get(cycle_handler_id)
   cycle_result = await cycle_handler.execute(pipeline=pipeline)
   if not cycle_result.success:
    raise HandlerError(cycle_handler_id, cycle_result.error)
   pipeline.final_judgment = cycle_result.output
   handlers_executed.append(cycle_handler_id)
   act_executor = self.registry.get("act_executor")
   recent_judgments = getattr(pipeline, "recent_judgments", None)
   act_result = await act_executor.execute(judgment=pipeline.final_judgment, pipeline=pipeline, recent_judgments=recent_judgments)
   if not act_result.success:
    raise HandlerError("act_executor", act_result.error)
   pipeline.action_result = act_result.output
   handlers_executed.append("act_executor")
   if selected_level.name == "MACRO":
    try:
     evolve_handler = self.registry.get("evolve")
     evolve_result = await evolve_handler.execute()
     if evolve_result.success:
      pipeline.meta_summary = evolve_result.output
      handlers_executed.append("evolve")
    except KeyError:
     pass
   try:
    budget_manager = self.registry.get("budget_manager")
    budget_result = await budget_manager.execute(budget_usd=budget_usd)
    if budget_result.success:
     pipeline.budget_status = budget_result.output
     handlers_executed.append("budget_manager")
   except KeyError:
    pass
   duration_ms = (time.perf_counter() - t0) * 1000
   return HandlerResult(success=True, handler_id="composer", output=pipeline.final_judgment, duration_ms=duration_ms, metadata={"handlers_executed": handlers_executed, "level": selected_level.name, "q_score": pipeline.final_judgment.q_score, "verdict": pipeline.final_judgment.verdict})
  except HandlerError as e:
   duration_ms = (time.perf_counter() - t0) * 1000
   return HandlerResult(success=False, handler_id="composer", error=f"Handler '{e.handler_id}' failed: {e.error}", duration_ms=duration_ms, metadata={"handlers_executed": handlers_executed, "failed_at": e.handler_id})
  except Exception as e:
   duration_ms = (time.perf_counter() - t0) * 1000
   logger.error(f"Unexpected error: {e}", exc_info=True)
   return HandlerResult(success=False, handler_id="composer", error=f"Unexpected error: {str(e)}", duration_ms=duration_ms, metadata={"handlers_executed": handlers_executed, "exception": e.__class__.__name__})
