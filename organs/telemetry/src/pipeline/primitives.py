import glob
import os
from datetime import datetime
from typing import Iterator

from organs.contracts.src.surface_contracts import iter_json_objects
from ..core.entities import LogEntry

class LogParser:
    """Primitives level: Fast parsing of raw files into structured entities."""
    
    @staticmethod
    def parse_antigravity_logs(brain_dir: str) -> Iterator[LogEntry]:
        pattern = os.path.join(os.path.expanduser(brain_dir), "*", ".system_generated", "logs", "transcript.jsonl")
        for filepath in glob.glob(pattern):
            session_id = filepath.split("/")[-4]
            with open(filepath, 'r', encoding='utf-8') as f:
                for data in iter_json_objects(f):
                    yield LogParser._map_to_entry("antigravity", session_id, data)

    @staticmethod
    def _map_to_entry(agent_id: str, session_id: str, data: dict) -> LogEntry:
        # Extract basic fields
        step_type = data.get("type", "UNKNOWN")
        content = data.get("content", "")
        
        # Tools
        tool_calls = []
        if "tool_calls" in data:
            for tc in data["tool_calls"]:
                tool_calls.append(tc)
                
        # Error detection primitive
        is_error = False
        if "Encountered error in step execution:" in content:
            is_error = True
            
        timestamp_str = data.get("created_at")
        try:
            ts = datetime.fromisoformat(timestamp_str.replace("Z", "+00:00")) if timestamp_str else datetime.now()
        except:
            ts = datetime.now()

        return LogEntry(
            agent_id=agent_id,
            session_id=session_id,
            timestamp=ts,
            step_type=step_type,
            content=content,
            tool_calls=tool_calls,
            is_error=is_error
        )
