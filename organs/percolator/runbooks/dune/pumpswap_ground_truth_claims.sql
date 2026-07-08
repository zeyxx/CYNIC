-- ==============================================================================
-- ODYSSEUM PITCH METRICS - GROUND TRUTH DES FRAIS RÉCLAMÉS (LTV)
-- ==============================================================================
-- Objectif : Obtenir la preuve cryptographique (Ground Truth) des montants 
-- réellement transférés aux créateurs lorsqu'ils appellent "collect_coin_creator_fee".
-- 
-- Mécanisme : 
-- 1. On isole les transactions où l'instruction Anchor "collect_coin_creator_fee" est appelée.
-- 2. On joint ces transactions avec la table des transferts de tokens (solana.token_transfers)
--    pour voir exactement combien de WSOL ou d'USDC a atterri dans le wallet du créateur.
-- ==============================================================================

WITH creator_fee_claims AS (
    SELECT 
        tx_id,
        signer AS creator_address,
        block_time
    FROM solana.instructions
    WHERE block_time >= current_date - interval '90' day
      AND executing_account = 'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA'
      -- Discriminator exact pour "collect_coin_creator_fee" (Le vrai B2B)
      AND substr(from_base58(data), 1, 8) = X'a039592ab58b2b42' 
),
fee_transfers AS (
    SELECT 
        c.creator_address,
        c.tx_id,
        t.token_mint_address,
        -- On convertit les unités natives (lamports et micro-USDC) en décimales humaines
        CASE 
            WHEN t.token_mint_address = 'So11111111111111111111111111111111111111112' THEN t.amount / 1e9
            WHEN t.token_mint_address = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' THEN t.amount / 1e6
            ELSE 0 
        END AS amount_normalized,
        CASE 
            WHEN t.token_mint_address = 'So11111111111111111111111111111111111111112' THEN 'SOL'
            WHEN t.token_mint_address = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' THEN 'USDC'
            ELSE 'OTHER'
        END AS pair_type
    FROM creator_fee_claims c
    JOIN solana.token_transfers t 
      ON c.tx_id = t.tx_id 
      AND t.to_address = c.creator_address
    WHERE t.block_time >= current_date - interval '90' day
      AND t.token_mint_address IN (
          'So11111111111111111111111111111111111111112', -- WSOL
          'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'  -- USDC
      )
),
creator_totals AS (
    SELECT 
        creator_address,
        pair_type,
        sum(amount_normalized) as total_claimed_by_creator
    FROM fee_transfers
    GROUP BY creator_address, pair_type
)
SELECT 
    pair_type,
    count(distinct creator_address) AS unique_creators,
    sum(total_claimed_by_creator) AS total_fees_claimed,
    avg(total_claimed_by_creator) AS average_ltv,
    approx_percentile(total_claimed_by_creator, 0.5) AS median_ltv
FROM creator_totals
GROUP BY pair_type;
