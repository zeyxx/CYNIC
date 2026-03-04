"""
CYNIC Metabolic Token Filter (RTK Doctrine).
Compresses raw string inputs (logs, code) into high-density signal before LLM ingestion.
"""
import re

class TokenFilter:
    @staticmethod
    def compress_shell_output(text: str, max_lines: int = 50) -> str:
        """Removes whitespace, deduplicates repeated logs, and truncates safely."""
        if not text:
            return ""
            
        lines = text.splitlines()
        filtered = []
        last_line = ""
        repeat_count = 0
        
        for line in lines:
            line = line.strip()
            if not line:
                continue
                
            if line == last_line:
                repeat_count += 1
                continue
            else:
                if repeat_count > 0:
                    filtered.append(f"... (repeated {repeat_count} times)")
                repeat_count = 0
                filtered.append(line)
                last_line = line
                
        if repeat_count > 0:
            filtered.append(f"... (repeated {repeat_count} times)")
            
        # Truncate middle if too long
        if len(filtered) > max_lines:
            half = max_lines // 2
            filtered = filtered[:half] + ["\n... [TRUNCATED METABOLIC NOISE] ...\n"] + filtered[-half:]
            
        return "\n".join(filtered)
