from __future__ import annotations

import unittest

from organs.solana.src.core.pump_live_probe import (
    PUMP_PROGRAM_ID,
    PUMPSWAP_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    MintProbe,
    classify_mint_probe,
)


class PumpLiveProbeTests(unittest.TestCase):
    def test_classifies_token_2022_pumpswap_sample(self) -> None:
        result = classify_mint_probe(
            MintProbe(
                mint="DMGvqNkhS3BtNDRypN6tEY6jpkatHJj4WQZtPmFzpump",
                account={
                    "result": {
                        "value": {
                            "owner": TOKEN_2022_PROGRAM_ID,
                            "data": {
                                "parsed": {
                                    "info": {
                                        "decimals": 6,
                                        "supply": "999977944581124",
                                        "isInitialized": True,
                                        "mintAuthority": None,
                                        "freezeAuthority": None,
                                    }
                                }
                            },
                        }
                    }
                },
                signatures=[{"signature": "sig1"}],
                transactions=[
                    {
                        "result": {
                            "transaction": {
                                "message": {
                                    "instructions": [
                                        {"programId": PUMPSWAP_PROGRAM_ID},
                                    ]
                                }
                            },
                            "meta": {"innerInstructions": []},
                        }
                    }
                ],
            )
        )

        self.assertEqual(result["classification"], "pumpswap_evidence_without_sampled_pump_origin")
        self.assertEqual(result["domain"], "solana")
        self.assertEqual(result["evidence"]["token_program_kind"], "token-2022")
        self.assertIsNone(result["mint_info"]["mint_authority"])

    def test_classifies_full_pump_and_pumpswap_evidence(self) -> None:
        result = classify_mint_probe(
            MintProbe(
                mint="9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump",
                account={"result": {"value": {"owner": TOKEN_2022_PROGRAM_ID}}},
                signatures=[],
                transactions=[
                    {
                        "result": {
                            "transaction": {
                                "message": {
                                    "instructions": [
                                        {"programId": PUMP_PROGRAM_ID},
                                        {"programId": PUMPSWAP_PROGRAM_ID},
                                    ]
                                }
                            }
                        }
                    }
                ],
            )
        )

        self.assertEqual(result["classification"], "pump_origin_with_pumpswap_evidence")
        self.assertEqual(result["confidence"], "observed")


if __name__ == "__main__":
    unittest.main()
