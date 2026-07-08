-- ============================================================================
-- ODYSSEUM B2B PITCH DECK METRICS - EXACT ON-CHAIN EXTRACTION
-- Source of Truth: canonical_pumpswap_ssot.json
-- Architecture: Single-Pass Master Query (Optimized for Trino Engine)
-- ============================================================================

WITH
-- 1. IDENTIFIER LES CREATIONS DE POOLS (Créateurs Solos Initiaux)
initial_creators AS (
    SELECT 
        account_arguments[6] AS coin_creator, -- Le client B2B initial
        MIN(block_time) AS first_seen_time
    FROM solana.instruction_calls
    WHERE executing_account = 'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA'
    AND block_date >= CURRENT_DATE - INTERVAL '120' day -- OPTIMISATION DUNE : Fenêtre glissante SaaS (120 jours)
    AND substr(data, 1, 8) = from_hex('e992d18ecf6840bc') -- EXCLUSIVEMENT create_pool
    AND tx_success = TRUE
    GROUP BY 1
),

-- 2. TRACER LES REVENUS B2B REELS (Les Claims des Créateurs)
creator_fee_claims AS (
    SELECT 
        tx_id,
        block_time,
        CASE 
            WHEN substr(data, 1, 8) = from_hex('a039592ab58b2b42') THEN account_arguments[3] -- collect_coin_creator_fee: index 3
            WHEN substr(data, 1, 8) = from_hex('01214eb921432c5c') THEN account_arguments[6] -- transfer_creator_fees_to_pump_v2: index 6
        END AS coin_creator,
        CASE 
            WHEN substr(data, 1, 8) = from_hex('a039592ab58b2b42') THEN 'collect_coin_creator_fee'
            WHEN substr(data, 1, 8) = from_hex('01214eb921432c5c') THEN 'transfer_creator_fees_to_pump_v2'
        END AS claim_type
    FROM solana.instruction_calls
    WHERE executing_account = 'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA'
    AND block_date >= CURRENT_DATE - INTERVAL '120' day -- OPTIMISATION DUNE : Fenêtre glissante SaaS (120 jours)
    AND substr(data, 1, 8) IN (
        from_hex('a039592ab58b2b42'), -- B2B Solo
        from_hex('01214eb921432c5c')  -- B2B Team (Sweep vers le main vault)
    )
    AND tx_success = TRUE
),

-- 3. QUANTIFIER LES TRANSFERTS FINANCIERS ($) LIES AUX CLAIMS (Adam_Tehc Technique)
financial_transfers_raw AS (
    SELECT 
        tx_id,
        SUM(COALESCE(amount_usd, 0)) AS amount_usd
    FROM tokens_solana.transfers
    WHERE block_date >= CURRENT_DATE - INTERVAL '120' day
    -- OPTIMISATION DUNE CRITIQUE : Filtre structurel (divise la RAM par 10 000)
    -- On ne lit QUE les transferts déclenchés par le programme PumpSwap, plutôt que tous les transferts WSOL/USDC de Solana
    AND outer_executing_account = 'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA'
    AND token_mint_address IN ('So11111111111111111111111111111111111111112', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB')
    -- LE SECRET ANTI-OOM DE DUNE : Dynamic IN-Filter
    AND tx_id IN (SELECT tx_id FROM creator_fee_claims)
    GROUP BY 1
),

-- Jointure propre post-filtrage
financial_transfers AS (
    SELECT 
        c.coin_creator,
        c.claim_type,
        c.block_time,
        f.amount_usd
    FROM creator_fee_claims c
    JOIN financial_transfers_raw f ON f.tx_id = c.tx_id
    WHERE c.coin_creator IS NOT NULL
),

-- 4. FUSIONNER LE TAM (Solos initiaux + PDAs mutés/équipes)
all_b2b_clients AS (
    SELECT coin_creator, MIN(first_seen_time) AS acquisition_date
    FROM (
        SELECT coin_creator, first_seen_time FROM initial_creators
        UNION ALL
        SELECT coin_creator, MIN(block_time) AS first_seen_time FROM financial_transfers GROUP BY 1
    )
    WHERE coin_creator IS NOT NULL
    GROUP BY 1
),

-- 5. AGREGATION PAR CLIENT B2B (LTV, Fréquence, Lifespan)
client_metrics AS (
    SELECT 
        a.coin_creator,
        a.acquisition_date,
        MAX(f.block_time) AS last_claim_date,
        COALESCE(SUM(f.amount_usd), 0) AS lifetime_value_usd, -- LTV du Client B2B
        COUNT(f.block_time) AS total_claims -- Stickiness
    FROM all_b2b_clients a
    LEFT JOIN financial_transfers f ON f.coin_creator = a.coin_creator
    GROUP BY 1, 2
),

-- 6. CALCUL DU WHALE DOMINANCE (Revenue Concentration)
concentration_calc AS (
    SELECT 
        SUM(lifetime_value_usd) AS total_arr,
        SUM(CASE WHEN ntile <= (SELECT COUNT(*) * 0.01 FROM client_metrics) THEN lifetime_value_usd ELSE 0 END) AS top_1_percent_arr
    FROM (
        SELECT lifetime_value_usd, NTILE(100) OVER (ORDER BY lifetime_value_usd DESC) as ntile
        FROM client_metrics
        WHERE lifetime_value_usd > 0
    ) ranked
)

-- 7. PITCH DECK MASTER OUTPUT (Les 7 KPIs B2B d'Odysseum)
SELECT 
    -- 1. TAM (Total Addressable Market = Nombre de Créateurs & PDAs d'équipes uniques)
    (SELECT COUNT(DISTINCT coin_creator) FROM client_metrics) AS tam_b2b_clients,
    
    -- 2. ARR (Total des revenus collectés par l'ensemble du marché B2B)
    (SELECT SUM(lifetime_value_usd) FROM client_metrics) AS arr_total_market_usd,
    
    -- 3. ARPU / Moyenne LTV par Client (Le chiffre clé du SaaS)
    (SELECT AVG(lifetime_value_usd) FROM client_metrics WHERE lifetime_value_usd > 0) AS arpu_average_ltv_usd,
    
    -- 4. LTV Médiane (Filtre les outliers / influenceurs géants)
    (SELECT approx_percentile(lifetime_value_usd, 0.5) FROM client_metrics WHERE lifetime_value_usd > 0) AS median_ltv_usd,
    
    -- 5. Lifespan & Churn (Durée d'engagement moyenne d'un client B2B)
    (SELECT AVG(date_diff('day', acquisition_date, last_claim_date)) FROM client_metrics WHERE total_claims > 0) AS avg_client_lifespan_days,
    
    -- 6. Frequency of Claims (Stickiness SaaS)
    (SELECT AVG(total_claims) FROM client_metrics WHERE total_claims > 0) AS avg_claims_per_client,
    
    -- 7. Revenue Concentration (La dépendance au top 1% des clients B2B)
    (SELECT (top_1_percent_arr / NULLIF(total_arr, 0)) * 100 FROM concentration_calc) AS revenue_concentration_percentage

FROM client_metrics
LIMIT 1;
