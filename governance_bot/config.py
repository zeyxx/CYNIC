"""
Governance Bot Configuration
"""

import os
from dotenv import load_dotenv

load_dotenv()

# Discord Bot
DISCORD_TOKEN = os.getenv("DISCORD_TOKEN", "YOUR_DISCORD_TOKEN_HERE")
DISCORD_PREFIX = "/"

# CYNIC Integration
CYNIC_URL = os.getenv("CYNIC_URL", "http://127.0.0.1:8765")
CYNIC_MCP_ENABLED = True
CYNIC_MCP_URL = os.getenv("CYNIC_MCP_URL", "http://127.0.0.1:8766")

# Database
DATABASE_URL = os.getenv("DATABASE_URL", "sqlite:///governance_bot.db")

# Community Settings (defaults)
DEFAULT_VOTING_PERIOD_HOURS = 72
DEFAULT_EXECUTION_DELAY_HOURS = 24
DEFAULT_QUORUM_PERCENTAGE = 25
DEFAULT_APPROVAL_THRESHOLD_PERCENTAGE = 50
DEFAULT_PROPOSAL_SUBMISSION_FEE = 100
DEFAULT_MIN_DOGS_CONSENSUS = 6

# Features
ENABLE_GASDF = True
ENABLE_NEAR_EXECUTION = False  # Set to True once smart contract is deployed
ENABLE_LEARNING_LOOP = True

# Logging
LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO")
LOG_FILE = "governance_bot.log"

# Bot Settings
BOT_ACTIVITY = "governance decisions"
BOT_STATUS = "ACTIVE"
