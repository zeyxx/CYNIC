-- Requête Dune stricte séparant mathématiquement la Bonding Curve et PumpSwap
-- par leurs adresses de Smart Contracts exactes.

WITH exact_trades AS (
    SELECT 
        block_time,
        tx_id,
        trader_id,
        amount_usd,
        CASE 
            WHEN token_bought_mint_address != 'So11111111111111111111111111111111111111112' 
            THEN token_bought_mint_address 
            ELSE token_sold_mint_address 
        END AS meme_token_address,
        
        -- Distinction stricte par le Smart Contract appelé lors du Trade
        CASE 
            WHEN project_contract_address = '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P' THEN 'Pump_Bonding_Curve'
            -- Adresse officielle du programme AMM PumpSwap issue de son IDL
            WHEN project_contract_address = 'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA' THEN 'PumpSwap_AMM'
            ELSE 'OTHER'
        END AS contract_phase

    FROM dex_solana.trades
    WHERE block_time >= current_date - interval '90' day
      -- On filtre uniquement sur les trades impliquant Pump.fun ou PumpSwap par adresse
      AND project_contract_address IN (
          '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P',
          'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA'
      )
),
phase_aggregations AS (
    SELECT 
        contract_phase,
        count(distinct meme_token_address) AS total_unique_tokens,
        sum(amount_usd) AS total_volume_usd,
        count(distinct tx_id) AS total_trades
    FROM exact_trades
    WHERE contract_phase IN ('Pump_Bonding_Curve', 'PumpSwap_AMM')
    GROUP BY contract_phase
)
SELECT 
    contract_phase,
    total_unique_tokens,
    total_volume_usd,
    total_trades,
    (total_volume_usd / total_unique_tokens) AS average_volume_per_token
FROM phase_aggregations;
