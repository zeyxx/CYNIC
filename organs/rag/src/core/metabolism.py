import json
from pathlib import Path
from datetime import datetime, timezone
import hashlib

class Document:
    def __init__(self, id: str, content: str, source: str, tags: list, timestamp: float):
        self.id = id
        self.content = content
        self.source = source
        self.tags = tags
        self.timestamp = timestamp
        self.decay_score = 0.0

class MetabolismEngine:
    """
    The Cybernetic Metabolism Engine (RAG Core).
    Ingests raw data (tweets, docs), sorts them, and applies a Garbage Collector (Sweeper).
    """
    def __init__(self, rag_store_path: str):
        self.store_path = Path(rag_store_path)
        self.store_path.parent.mkdir(parents=True, exist_ok=True)
        self.documents = self._load_store()

    def _load_store(self) -> dict:
        if not self.store_path.exists():
            return {}
        with open(self.store_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
            return {k: Document(**v) for k, v in data.items()}

    def _save_store(self):
        with open(self.store_path, 'w', encoding='utf-8') as f:
            json.dump({k: vars(v) for k, v in self.documents.items()}, f, indent=2)

    def ingest_twitter_context(self, jsonl_path: str):
        """Ingests raw JSONL from organ-twitter into the RAG memory."""
        path = Path(jsonl_path)
        if not path.exists():
            return 0
        
        count = 0
        with open(path, 'r', encoding='utf-8') as f:
            for line in f:
                if not line.strip(): continue
                data = json.loads(line)
                item = data.get("item", {})
                doc_id = item.get("item_id", hashlib.md5(line.encode()).hexdigest())
                
                # Deduplication and insertion
                if doc_id not in self.documents:
                    self.documents[doc_id] = Document(
                        id=doc_id,
                        content=item.get("text", ""),
                        source=item.get("url", "organ-twitter"),
                        tags=item.get("tags", []),
                        timestamp=datetime.now(timezone.utc).timestamp()
                    )
                    count += 1
        
        self.garbage_collect()
        self._save_store()
        return count

    def garbage_collect(self):
        """
        The Central Garbage Collector (Sweeper).
        Identifies obsolete documents, chaotic duplicates, and decays their relevance.
        If decay_score > 1.0, the document is archived/purged.
        """
        current_time = datetime.now(timezone.utc).timestamp()
        keys_to_purge = []
        
        for doc_id, doc in self.documents.items():
            age_days = (current_time - doc.timestamp) / (3600 * 24)
            
            # 1. Temporal Decay (Old tech tweets decay faster than foundational docs)
            if "inference-sota" in doc.tags and age_days > 30:
                doc.decay_score += 0.5  # AI moves fast, 1 month is old
            
            # 2. Chaos / Deprecation Flag
            if "deprecated" in doc.tags:
                doc.decay_score += 1.0
                
            # 3. Purge threshold
            if doc.decay_score >= 1.0:
                keys_to_purge.append(doc_id)
                
        # Execute Purge (Sweeping the dead cells)
        for k in keys_to_purge:
            del self.documents[k]
        
        return len(keys_to_purge)

if __name__ == "__main__":
    engine = MetabolismEngine(".cynic/organs/rag/store.json")
    print("Metabolism Engine initialized.")
