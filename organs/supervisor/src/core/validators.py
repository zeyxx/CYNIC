# Tier 1
import re
from typing import List, Tuple, Dict
from pathlib import Path
from .ports import VersionControlPort, DocsSourcePort
import collections

VOLATILE_FILE_REGEX = re.compile(
    r"^(infra/organ-anvil/(state\.json|audit\.jsonl)|organs/[^/]+/state/.*\.(json|jsonl))$"
)

def find_staged_volatile_files(vcs: VersionControlPort) -> List[str]:
    staged_files = vcs.get_staged_files()
    return [f for f in staged_files if f and VOLATILE_FILE_REGEX.match(f)]

def find_hardcoded_absolute_paths(vcs: VersionControlPort) -> Dict[str, List[str]]:
    files_to_check = vcs.get_staged_files()
    violations = {}
    path_regex = re.compile(r"/home/|/Users/")
    for file_path in files_to_check:
        try:
            with open(file_path, "r", errors="ignore") as f:
                lines = f.readlines()
            for i, line in enumerate(lines):
                if path_regex.search(line):
                    if file_path not in violations:
                        violations[file_path] = []
                    violations[file_path].append(f"{i+1}:{line.strip()}")
        except FileNotFoundError:
            continue
        except Exception as e:
            print(f"Could not read {file_path}: {e}")
    return violations

def find_unlogged_ok_calls(vcs: VersionControlPort) -> Dict[str, List[str]]:
    violations = {}
    rust_files = [f for f in vcs.get_staged_files() if f.endswith(".rs")]
    exclusion_patterns = [re.compile(p) for p in [r"//", r"#\[cfg\(test\)\]", r"mod tests", r"parse\(\)\.ok\(\)", r"env::var.*\.ok\(\)", r"inspect_err", r"to_str\(\)\.ok\(\)", r"from_utf8.*\.ok\(\)", r"filter_map.*\.ok\(\)", r"create_dir_all.*\.ok\(\)", r"remove_file.*\.ok\(\)", r"\.lock\(\)\.ok\(\)", r"\.read\(\)\.ok\(\)", r"\.output\(\)", r"read_to_string.*\.ok\(\)"]]
    logging_keywords = re.compile(r"inspect_err|tracing::|eprintln!|warn!|env::var|WHY:|K14|poison")
    for file_path in rust_files:
        if "probe/" in file_path or "probes/" in file_path:
            continue
        try:
            with open(file_path, "r", errors="ignore") as f:
                lines = f.readlines()
            for i, line in enumerate(lines):
                if ".ok()" in line:
                    if any(p.search(line) for p in exclusion_patterns):
                        continue
                    start = max(0, i - 8)
                    end = min(len(lines), i + 4)
                    vicinity = "".join(lines[start:end])
                    if not logging_keywords.search(vicinity):
                        if file_path not in violations:
                            violations[file_path] = []
                        violations[file_path].append(f"{i+1}:{line.strip()}")
        except FileNotFoundError:
            continue
        except Exception as e:
            print(f"Could not read {file_path}: {e}")
    return violations

def sync_mcp_tools_docs(docs_source: DocsSourcePort) -> Tuple[int, int]:
    exposure_data = docs_source.read_exposure_data()
    if not exposure_data:
        return 0, 0
    header = "| Tool | What | When |\n|------|------|------|\n"
    all_tools = {}
    for server in exposure_data.get("mcp_servers", []):
        for tool in server.get("tools", []):
            tool_name = tool.get("name")
            if tool_name and tool_name not in all_tools:
                description = f"Tool from {server.get('name', 'unknown server')}"
                all_tools[tool_name] = f"| `{tool_name}` | {description} | As needed |"
    rows = [all_tools[name] for name in sorted(all_tools.keys())]
    tools_table = header + "\n".join(rows)
    content = docs_source.read_agents_md()
    pattern = r"(<!-- BEGIN_MCP_TOOLS -->\n)(.*?)(\n<!-- END_MCP_TOOLS -->)"
    new_content, count = re.subn(pattern, f"\\1{tools_table}\\3", content, flags=re.DOTALL)
    if count > 0:
        docs_source.write_agents_md(new_content)
    return len(all_tools), count

def find_disallowed_cfg_in_domain(project_root: Path) -> Dict[str, List[str]]:
    violations = {}
    domain_path = project_root / "crates/cynic-kernel/src/domain"
    if not domain_path.is_dir():
        print(f"Warn: Domain directory not found at {domain_path}")
        return violations
    exclusion_patterns = [re.compile(p) for p in [r"//", r"#\[cfg\(test\)\]", r"#\[cfg_attr\(test"]]
    cfg_regex = re.compile(r"#\[cfg\(");
    for file_path in domain_path.rglob("*.rs"):
        try:
            with open(file_path, "r", errors="ignore") as f:
                lines = f.readlines()
            for i, line in enumerate(lines):
                if cfg_regex.search(line):
                    if any(p.search(line) for p in exclusion_patterns):
                        continue
                    if str(file_path) not in violations:
                        violations[str(file_path)] = []
                    violations[str(file_path)].append(f"{i+1}:{line.strip()}")
        except Exception as e:
            print(f"Could not read {file_path}: {e}")
    return violations

def find_misplaced_reqwest_usage(project_root: Path) -> Dict[str, List[str]]:
    violations = {}
    search_paths = [project_root / "crates/cynic-kernel/src/domain", project_root / "crates/cynic-kernel/src/api", project_root / "crates/cynic-kernel/src/main.rs"]
    exclusion_file = "crates/cynic-kernel/src/api/mcp/proxy.rs"
    for path in search_paths:
        files_to_check = [path] if path.is_file() else path.rglob("*.rs") if path.is_dir() else []
        for file_path in files_to_check:
            if str(file_path).endswith(exclusion_file):
                continue
            try:
                with open(file_path, "r", errors="ignore") as f:
                    lines = f.readlines()
                for i, line in enumerate(lines):
                    if "reqwest" in line and not line.strip().startswith("//") and "K2-exempt" not in line:
                        if str(file_path) not in violations:
                            violations[str(file_path)] = []
                        violations[str(file_path)].append(f"{i+1}:{line.strip()}")
            except Exception as e:
                print(f"Could not read {file_path}: {e}")
    return violations

def find_reimplemented_pipeline_functions(project_root: Path) -> Dict[str, List[str]]:
    violations = {}
    pipeline_fns = ["format_crystal_context", "compute_qscore", "trimmed_mean"]
    api_path = project_root / "crates/cynic-kernel/src/api"
    if not api_path.is_dir():
        return violations
    for fn_name in pipeline_fns:
        fn_regex = re.compile(fr"fn\s+{fn_name}")
        for file_path in api_path.rglob("*.rs"):
            try:
                with open(file_path, "r", errors="ignore") as f:
                    for i, line in enumerate(f):
                        if fn_regex.search(line) and not line.strip().startswith("//"):
                            if str(file_path) not in violations:
                                violations[str(file_path)] = []
                            violations[str(file_path)].append(f"{i+1}:{line.strip()}")
            except Exception as e:
                print(f"Could not read {file_path}: {e}")
    return violations

def find_duplicate_trait_names(project_root: Path) -> List[str]:
    domain_path = project_root / "crates/cynic-kernel/src/domain"
    if not domain_path.is_dir():
        return []
    trait_regex = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?trait\s+(\w+)", re.MULTILINE)
    traits = []
    for file_path in domain_path.rglob("*.rs"):
        try:
            with open(file_path, "r", errors="ignore") as f:
                content = f.read()
                traits.extend(trait_regex.findall(content))
        except Exception as e:
            print(f"Could not read {file_path}: {e}")
    return [item for item, count in collections.Counter(traits).items() if count > 1]

def find_infra_type_leakage(project_root: Path) -> Dict[str, List[str]]:
    violations = {}
    domain_path = project_root / "crates/cynic-kernel/src/domain"
    if not domain_path.is_dir():
        return violations
    leakage_regex = re.compile(r"serde_json::Value|reqwest::|surrealdb::|axum::")
    for file_path in domain_path.rglob("*.rs"):
        if "#[cfg(test)]" in str(file_path):
            continue
        try:
            with open(file_path, "r", errors="ignore") as f:
                for i, line in enumerate(f):
                    if leakage_regex.search(line) and not line.strip().startswith("//"):
                        if str(file_path) not in violations:
                            violations[str(file_path)] = []
                        violations[str(file_path)].append(f"{i+1}:{line.strip()}")
        except Exception as e:
            print(f"Could not read {file_path}: {e}")
    return violations

def find_allow_without_why(project_root: Path) -> Dict[str, List[str]]:
    violations = {}
    kernel_path = project_root / "crates/cynic-kernel/src"
    if not kernel_path.is_dir():
        return violations
    allow_regex = re.compile(r"#\[allow\(");
    for file_path in kernel_path.rglob("*.rs"):
        if "#[cfg(test)]" in str(file_path) or "#[cfg_attr(test" in str(file_path):
            continue
        try:
            with open(file_path, "r", errors="ignore") as f:
                lines = f.readlines()
            for i, line in enumerate(lines):
                if allow_regex.search(line):
                    has_why = False
                    for j in range(max(0, i - 2), min(len(lines), i + 3)):
                        if "WHY:" in lines[j]:
                            has_why = True
                            break
                    if not has_why:
                        if str(file_path) not in violations:
                             violations[str(file_path)] = []
                        violations[str(file_path)].append(f"{i+1}:{line.strip()}")
        except Exception as e:
            print(f"Could not read {file_path}: {e}")
    return violations

def find_hardcoded_secrets(vcs: VersionControlPort) -> Dict[str, List[str]]:
    staged_content = vcs.get_staged_content()
    violations = {"RAW_SECRETS": [], "HARDCODED_CREDS": []}
    raw_secrets_regex = re.compile(r"AIza[\w-]{10,}|hf_[\w-]{10,}|sk-[\w-]{10,}")
    creds_regex = re.compile(r"""(api[_-]?key|token|password|secret)[^\w]*[:=]\s*['"]?[^\s$<'"]{6,}""")
    creds_exclusion_regex = re.compile(r"change-me|your-api-key-here|example|placeholder|dummy|fake|test-key|test-token|token-abc|redacted|already-present|prompt|CancellationToken|JUPYTER_TOKEN|PVEAPIToken|session_token|app_token|secretapikey|apikey|surreal_pass|token\?:\s*string|os\.environ\.get|os\.getenv|getpass\.getpass|args\.api_key|runtime\.API_KEY|globals\(\)\[\\\"API_KEY\\\"\]|get_api_key\(\)|get_x_credentials\(\)|password=password|password\s*= self\._build_password|\"password\": password|token = runtime\.authorize\(\)|_cf_credentials\(\)|result\.get|payload\.get|self\.api_key\s*=|api_key=api_key")
    for line in staged_content.split('\\n'):
        if line.startswith('+') and not line.startswith('+++'):
            line_content = line[1:]
            if raw_secrets_regex.search(line_content):
                violations["RAW_SECRETS"].append(line_content)
            if creds_regex.search(line_content) and not creds_exclusion_regex.search(line_content):
                violations["HARDCODED_CREDS"].append(line_content)
    return violations

def find_hardcoded_ips(vcs: VersionControlPort, project_root: Path) -> Dict[str, List[str]]:
    staged_content = vcs.get_staged_content()
    violations = {"STAGED_IPS": [], "BACKEND_IPS": []}
    ip_regex = re.compile(r"100\.(74|75|119)\.\d{1,3}\.\d{1,3}")
    for line in staged_content.split('\\n'):
        if line.startswith('+') and not line.startswith('+++'):
            if ip_regex.search(line[1:]):
                violations["STAGED_IPS"].append(line[1:])
    backends_path = project_root / "backends.toml"
    if backends_path.exists():
        try:
            with open(backends_path, "r") as f:
                for line in f:
                    if ip_regex.search(line):
                        violations["BACKEND_IPS"].append(line.strip())
        except Exception as e:
            print(f"Could not read {backends_path}: {e}")
    return violations
