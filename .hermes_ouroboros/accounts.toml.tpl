# Hermes X Accounts Configuration (template)
# Source: .hermes_ouroboros/accounts.toml.tpl
# Runtime: ~/.config/cynic/accounts.toml (LOCAL-ONLY, never committed)
#
# This is a TEMPLATE showing required structure. Copy to ~/.config/cynic/accounts.toml
# and fill in with real account credentials. The runtime file is NOT tracked in git.
#
# Each account needs:
#   - username: X.com handle (no @ symbol)
#   - password_env: Name of env var holding password (typically HERMES_X_PASSWORD or CYNIC_X_PASSWORD)
#   - profile: Chrome profile path (for account-specific cookies/state)
#   - resume_on_failure: Whether to use this as fallback if primary fails (optional, default false)

[accounts.cynic]
username = "your_handle_here"  # REQUIRED: Replace with real X.com handle
password_env = "CYNIC_X_PASSWORD"  # Env var name where password is stored
profile = "~/.cynic/organs/hermes/x/profiles/cynic"
resume_on_failure = false

# Additional accounts (optional)
# [accounts.personal]
# username = "your_personal_handle"
# password_env = "HERMES_X_PASSWORD"
# profile = "~/.cynic/organs/hermes/x/profiles/personal"
# resume_on_failure = true  # Use this account if cynic fails
