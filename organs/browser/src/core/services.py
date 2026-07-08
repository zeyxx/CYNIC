import datetime
from typing import List, Dict, Any, Optional
from .entities import NavigationIntent, PageEvidence, EvidenceItem, HumanGate
from .ports import (
    SearchPort,
    ScrapePort,
    BrowserPort,
    EvidenceStorePort,
    SecretScannerPort,
    KernelObservationPort,
)

class BrowserOrganService:
    def __init__(
        self,
        search_port: SearchPort,
        scrape_port: ScrapePort,
        browser_port: BrowserPort,
        evidence_store: EvidenceStorePort,
        secret_scanner: SecretScannerPort,
        kernel_obs: KernelObservationPort,
    ):
        self.search_port = search_port
        self.scrape_port = scrape_port
        self.browser_port = browser_port
        self.evidence_store = evidence_store
        self.secret_scanner = secret_scanner
        self.kernel_obs = kernel_obs

    def execute_task(self, intent: NavigationIntent) -> PageEvidence:
        # Check security policy for mutation risk
        is_mutation_task = any(
            act in ["submit", "post", "connect wallet", "sign", "purchase", "stake", "deposit"]
            for act in intent.allowed_actions
        )
        
        # Log start of the action
        self.evidence_store.log_action({
            "task_id": intent.id,
            "action": "start_task",
            "timestamp": datetime.datetime.now().isoformat(),
            "url": intent.url,
            "objective": intent.objective,
        })

        if is_mutation_task and not intent.human_gate.required:
            evidence = PageEvidence(
                task_id=intent.id,
                status="blocked",
                observed_at=datetime.datetime.now(),
                url=intent.url,
                blockers=["Mutation requested but no explicit human gate configuration provided"],
                summary="Blocked task execution to prevent un-gated mutations on external platforms."
            )
            self.evidence_store.save_evidence(intent.id, evidence)
            self.kernel_obs.observe(
                agent_id=intent.source,
                title="Mutation Blocked",
                content={"reason": "No human gate provided for mutating action"},
                domain="browser"
            )
            return evidence

        try:
            # Route target based on intent properties
            if intent.url.startswith("search://") or intent.url.startswith("searx://"):
                query = intent.context.get("query", intent.objective)
                results = self.search_port.search(query)
                evidence_items = []
                for res in results:
                    sanitized_title = self.secret_scanner.sanitize(res.get("title", ""))
                    sanitized_content = self.secret_scanner.sanitize(res.get("content", ""))
                    sanitized_url = self.secret_scanner.sanitize(res.get("url", ""))
                    evidence_items.append(EvidenceItem(
                        kind="search_result",
                        value=f"{sanitized_title} - {sanitized_url}: {sanitized_content}"
                    ))
                evidence = PageEvidence(
                    task_id=intent.id,
                    status="completed",
                    observed_at=datetime.datetime.now(),
                    url=intent.url,
                    evidence=evidence_items,
                    summary=f"Discovered {len(results)} candidate URLs for query: {query}"
                )
            elif "scrape" in intent.allowed_actions and "interact" not in intent.allowed_actions:
                raw_data = self.scrape_port.scrape(intent.url)
                markdown = self.secret_scanner.sanitize(raw_data.get("markdown", ""))
                metadata = raw_data.get("metadata", {})
                
                evidence = PageEvidence(
                    task_id=intent.id,
                    status="completed",
                    observed_at=datetime.datetime.now(),
                    url=intent.url,
                    evidence=[
                        EvidenceItem(kind="markdown", value=markdown),
                        EvidenceItem(kind="metadata", value=str(metadata))
                    ],
                    summary=f"Scraped {intent.url} using extraction layer."
                )
            else:
                evidence = self.browser_port.interact(intent)
                # Sanitize all items in page evidence
                for item in evidence.evidence:
                    item.value = self.secret_scanner.sanitize(item.value)
                evidence.summary = self.secret_scanner.sanitize(evidence.summary or "")

            self.evidence_store.save_evidence(intent.id, evidence)
            self.evidence_store.log_action({
                "task_id": intent.id,
                "action": "complete_task",
                "timestamp": datetime.datetime.now().isoformat(),
                "status": evidence.status,
                "evidence_count": len(evidence.evidence)
            })
            
            # Send observation to kernel
            self.kernel_obs.observe(
                agent_id=intent.source,
                title=f"Browser Observation for {intent.id}",
                content={
                    "task_id": evidence.task_id,
                    "url": evidence.url,
                    "status": evidence.status,
                    "summary": evidence.summary,
                    "mutations_performed": evidence.mutations_performed,
                    "blockers": evidence.blockers,
                },
                domain="browser"
            )
            return evidence

        except Exception as e:
            evidence = PageEvidence(
                task_id=intent.id,
                status="failed",
                observed_at=datetime.datetime.now(),
                url=intent.url,
                blockers=[str(e)],
                summary=f"Browser task execution failed: {str(e)}"
            )
            self.evidence_store.save_evidence(intent.id, evidence)
            self.evidence_store.log_action({
                "task_id": intent.id,
                "action": "fail_task",
                "timestamp": datetime.datetime.now().isoformat(),
                "error": str(e)
            })
            return evidence
