WITH pumpswap_trades AS (
    SELECT 
        block_time,
        trader_id,
        token_bought_mint_address,
        token_sold_mint_address,
        token_bought_amount,
        token_sold_amount,
        amount_usd,
        -- Identifier la paire : SOL ou USDC
        CASE 
            WHEN token_bought_mint_address = 'So11111111111111111111111111111111111111112' OR token_sold_mint_address = 'So11111111111111111111111111111111111111112' THEN 'SOL'
            WHEN token_bought_mint_address = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' OR token_sold_mint_address = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' THEN 'USDC'
            ELSE 'OTHER'
        END AS pair_type
    FROM dex_solana.trades
    WHERE project IN ('pumpswap') -- Ciblage exclusif de l'AMM post-graduation de Pump
      AND block_time >= current_date - interval '90' day
),
trade_metrics AS (
    SELECT 
        block_time,
        trader_id,
        pair_type,
        amount_usd,
        -- On isole l'adresse du token communautaire (celui qui n'est ni SOL ni USDC)
        CASE 
            WHEN token_bought_mint_address != 'So11111111111111111111111111111111111111112' 
                 AND token_bought_mint_address != 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' 
            THEN token_bought_mint_address 
            ELSE token_sold_mint_address 
        END AS meme_token_address,
        
        -- Récupération du volume dans la monnaie de la paire (Base Token)
        CASE 
            WHEN pair_type = 'SOL' AND token_bought_mint_address = 'So11111111111111111111111111111111111111112' THEN token_bought_amount
            WHEN pair_type = 'SOL' AND token_sold_mint_address = 'So11111111111111111111111111111111111111112' THEN token_sold_amount
            WHEN pair_type = 'USDC' AND token_bought_mint_address = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' THEN token_bought_amount
            WHEN pair_type = 'USDC' AND token_sold_mint_address = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' THEN token_sold_amount
            ELSE 0
        END AS base_amount,

        -- Récupération du volume du Meme Token
        CASE 
            WHEN token_bought_mint_address != 'So11111111111111111111111111111111111111112' 
                 AND token_bought_mint_address != 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' 
            THEN token_bought_amount 
            ELSE token_sold_amount 
        END AS meme_amount

    FROM pumpswap_trades
    WHERE pair_type IN ('SOL', 'USDC')
),
mcap_calculations AS (
    SELECT 
        *,
        -- Calcul strict du Marketcap dans les unités natives du Smart Contract
        -- Pour SOL : Marketcap en Lamports = (base_amount / meme_amount) * 1 Milliard (supply) * 10^9 (decimals SOL)
        -- Pour USDC : Marketcap en micro-USDC = (base_amount / meme_amount) * 1 Milliard (supply) * 10^6 (decimals USDC)
        CASE 
            WHEN pair_type = 'SOL' THEN (base_amount / NULLIF(meme_amount, 0)) * 1000000000 * 1000000000
            WHEN pair_type = 'USDC' THEN (base_amount / NULLIF(meme_amount, 0)) * 1000000000 * 1000000
            ELSE 0
        END AS mcap_native_units
    FROM trade_metrics
    WHERE meme_amount > 0 AND base_amount > 0
),
fee_tiers AS (
    SELECT 
        *,
        -- Matrice de frais dynamique Odysseum / PumpSwap (en BPS -> / 10000 pour avoir le taux décimal)
        CASE 
            -- === BARÈME PAIRE SOL (en Lamports) ===
            WHEN pair_type = 'SOL' AND mcap_native_units < 420000000000 THEN 0.0030
            WHEN pair_type = 'SOL' AND mcap_native_units < 1470000000000 THEN 0.0095
            WHEN pair_type = 'SOL' AND mcap_native_units < 2460000000000 THEN 0.0090
            WHEN pair_type = 'SOL' AND mcap_native_units < 3440000000000 THEN 0.0085
            WHEN pair_type = 'SOL' AND mcap_native_units < 4420000000000 THEN 0.0080
            WHEN pair_type = 'SOL' AND mcap_native_units < 9820000000000 THEN 0.0075
            WHEN pair_type = 'SOL' AND mcap_native_units < 14740000000000 THEN 0.0070
            WHEN pair_type = 'SOL' AND mcap_native_units < 19650000000000 THEN 0.0065
            WHEN pair_type = 'SOL' AND mcap_native_units < 24560000000000 THEN 0.0060
            WHEN pair_type = 'SOL' AND mcap_native_units < 29470000000000 THEN 0.0055
            WHEN pair_type = 'SOL' AND mcap_native_units < 34380000000000 THEN 0.0050
            WHEN pair_type = 'SOL' AND mcap_native_units < 39300000000000 THEN 0.0045
            WHEN pair_type = 'SOL' AND mcap_native_units < 44210000000000 THEN 0.0040
            WHEN pair_type = 'SOL' AND mcap_native_units < 49120000000000 THEN 0.0035
            WHEN pair_type = 'SOL' AND mcap_native_units < 54030000000000 THEN 0.0030
            WHEN pair_type = 'SOL' AND mcap_native_units < 58940000000000 THEN 0.00275
            WHEN pair_type = 'SOL' AND mcap_native_units < 63860000000000 THEN 0.0025
            WHEN pair_type = 'SOL' AND mcap_native_units < 68770000000000 THEN 0.00225
            WHEN pair_type = 'SOL' AND mcap_native_units < 73680000000000 THEN 0.0020
            WHEN pair_type = 'SOL' AND mcap_native_units < 78590000000000 THEN 0.00175
            WHEN pair_type = 'SOL' AND mcap_native_units < 83500000000000 THEN 0.0015
            WHEN pair_type = 'SOL' AND mcap_native_units < 88400000000000 THEN 0.00125
            WHEN pair_type = 'SOL' AND mcap_native_units < 93330000000000 THEN 0.0010
            WHEN pair_type = 'SOL' AND mcap_native_units < 98240000000000 THEN 0.00075
            WHEN pair_type = 'SOL' THEN 0.0005
            
            -- === BARÈME PAIRE USDC (en micro-USDC) ===
            WHEN pair_type = 'USDC' AND mcap_native_units < 85000000000 THEN 0.0030
            WHEN pair_type = 'USDC' AND mcap_native_units < 300000000000 THEN 0.0095
            WHEN pair_type = 'USDC' AND mcap_native_units < 500000000000 THEN 0.0090
            WHEN pair_type = 'USDC' AND mcap_native_units < 700000000000 THEN 0.0085
            WHEN pair_type = 'USDC' AND mcap_native_units < 900000000000 THEN 0.0080
            WHEN pair_type = 'USDC' AND mcap_native_units < 2000000000000 THEN 0.0075
            WHEN pair_type = 'USDC' AND mcap_native_units < 3000000000000 THEN 0.0070
            WHEN pair_type = 'USDC' AND mcap_native_units < 4000000000000 THEN 0.0065
            WHEN pair_type = 'USDC' AND mcap_native_units < 5000000000000 THEN 0.0060
            WHEN pair_type = 'USDC' AND mcap_native_units < 6000000000000 THEN 0.0055
            WHEN pair_type = 'USDC' AND mcap_native_units < 7000000000000 THEN 0.0050
            WHEN pair_type = 'USDC' AND mcap_native_units < 8000000000000 THEN 0.0045
            WHEN pair_type = 'USDC' AND mcap_native_units < 9000000000000 THEN 0.0040
            WHEN pair_type = 'USDC' AND mcap_native_units < 10000000000000 THEN 0.0035
            WHEN pair_type = 'USDC' AND mcap_native_units < 11000000000000 THEN 0.0030
            WHEN pair_type = 'USDC' AND mcap_native_units < 12000000000000 THEN 0.00275
            WHEN pair_type = 'USDC' AND mcap_native_units < 13000000000000 THEN 0.0025
            WHEN pair_type = 'USDC' AND mcap_native_units < 14000000000000 THEN 0.00225
            WHEN pair_type = 'USDC' AND mcap_native_units < 15000000000000 THEN 0.0020
            WHEN pair_type = 'USDC' AND mcap_native_units < 16000000000000 THEN 0.00175
            WHEN pair_type = 'USDC' AND mcap_native_units < 17000000000000 THEN 0.0015
            WHEN pair_type = 'USDC' AND mcap_native_units < 18000000000000 THEN 0.00125
            WHEN pair_type = 'USDC' AND mcap_native_units < 19000000000000 THEN 0.0010
            WHEN pair_type = 'USDC' AND mcap_native_units < 20000000000000 THEN 0.00075
            WHEN pair_type = 'USDC' THEN 0.0005
            ELSE 0 
        END AS dynamic_creator_fee_rate
    FROM mcap_calculations
),
token_aggregations AS (
    SELECT 
        meme_token_address,
        pair_type,
        -- Revenu Odysseum perçu sur chaque trade = volume_USD * taux_dynamique
        sum(amount_usd * dynamic_creator_fee_rate) AS total_creator_fee_usd,
        count(distinct trader_id) AS unique_traders,
        date_diff('day', min(block_time), max(block_time)) AS lifespan_days
    FROM fee_tiers
    GROUP BY meme_token_address, pair_type
)
SELECT 
    pair_type,
    count(meme_token_address) AS total_pumpswap_communities,
    avg(lifespan_days) AS avg_lifespan_days_on_pumpswap,
    avg(unique_traders) AS avg_unique_traders,
    sum(total_creator_fee_usd) AS total_market_fees_usd,
    avg(total_creator_fee_usd) AS average_ltv_usd,
    approx_percentile(total_creator_fee_usd, 0.5) AS median_ltv_usd
FROM token_aggregations
GROUP BY pair_type;
