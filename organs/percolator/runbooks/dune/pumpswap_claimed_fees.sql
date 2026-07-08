-- ==============================================================================
-- ODYSSEUM PITCH METRICS - LES "CLAIMED FEES" (LE VRAI ARR B2B)
-- ==============================================================================
-- Objectif : Prouver la rétention B2B en mesurant l'argent REELLEMENT retiré
-- par les communautés.
-- Méthode : On scanne directement les instructions de retrait sur pAMMBay...
--
-- Discriminators Anchor :
--   - claim_cashback : X'253a237ebe35e4c5'
--   - claim_token_incentives : X'1004471ccc01281b'
--   - collect_coin_creator_fee : X'a039592ab58b2b42'
-- ==============================================================================

WITH claim_instructions AS (
    SELECT 
        block_time,
        tx_id,
        executing_account AS program_id,
        signer AS creator_address,
        CASE 
            WHEN substr(from_base58(data), 1, 8) = X'253a237ebe35e4c5' THEN 'claim_cashback'
            WHEN substr(from_base58(data), 1, 8) = X'1004471ccc01281b' THEN 'claim_token_incentives'
            WHEN substr(from_base58(data), 1, 8) = X'a039592ab58b2b42' THEN 'collect_coin_creator_fee'
        END AS claim_type
    FROM solana.instructions
    WHERE block_time >= current_date - interval '90' day
      AND executing_account = 'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA'
      AND (
          substr(from_base58(data), 1, 8) = X'253a237ebe35e4c5'
       OR substr(from_base58(data), 1, 8) = X'1004471ccc01281b'
       OR substr(from_base58(data), 1, 8) = X'a039592ab58b2b42'
      )
)
SELECT 
    claim_type,
    count(tx_id) AS total_claim_transactions,
    count(distinct creator_address) AS total_unique_creators_claiming,
    min(block_time) AS first_claim,
    max(block_time) AS last_claim
FROM claim_instructions
GROUP BY claim_type
ORDER BY total_claim_transactions DESC;
