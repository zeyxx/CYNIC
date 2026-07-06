mod tests {
    use super::*;

    // A test Dog that returns fixed scores
    struct FixedDog {
        name: String,
        scores: AxiomScores,
    }

    #[async_trait::async_trait]
    impl Dog for FixedDog {
        fn id(&self) -> &str {
            &self.name
        }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Ok(self.scores.clone())
        }
    }

    struct FailDog;

    #[async_trait::async_trait]
    impl Dog for FailDog {
        fn id(&self) -> &str {
            "fail-dog"
        }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Err(DogError::ApiError("test failure".into()))
        }
    }

    /// Convenience: build Judge with auto-created CircuitBreaker per Dog (test-only).
    fn test_judge(dogs: Vec<Arc<dyn Dog>>) -> Judge {
        let breakers: Vec<Arc<dyn HealthGate>> = dogs
            .iter()
            .map(|d| {
                Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(
                    d.id().to_string(),
                )) as Arc<dyn HealthGate>
            })
            .collect();
        Judge::new(dogs, breakers)
    }

    fn test_metrics() -> Metrics {
        Metrics::new()
    }

    fn test_stimulus() -> Stimulus {
        Stimulus {
            content: "e4 e5 Nf3".into(),
            context: None,
            domain: Some("chess".into()),
            request_id: None,
        }
    }

    #[tokio::test]
    async fn single_dog_produces_verdict() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "test".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        assert!(verdict.q_score.total <= PHI_INV + 1e-10);
        assert!(verdict.q_score.total > 0.0);
        assert_eq!(verdict.dog_id, "test");
        assert_eq!(verdict.dog_scores.len(), 1);
        assert!(!verdict.anomaly_detected);
    }

    #[tokio::test]
    async fn multiple_dogs_averaged() {
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "high".into(),
                scores: AxiomScores {
                    fidelity: 0.8,
                    phi: 0.8,
                    verify: 0.8,
                    culture: 0.8,
                    burn: 0.8,
                    sovereignty: 0.8,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "low".into(),
                scores: AxiomScores {
                    fidelity: 0.2,
                    phi: 0.2,
                    verify: 0.2,
                    culture: 0.2,
                    burn: 0.2,
                    sovereignty: 0.2,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        assert!(verdict.dog_id.contains("high"));
        assert!(verdict.dog_id.contains("low"));
        assert!(verdict.q_score.total > 0.3);
        assert!(verdict.q_score.total < 0.55);
        assert_eq!(verdict.dog_scores.len(), 2);
    }

    #[tokio::test]
    async fn surviving_dog_still_produces_verdict() {
        let judge = test_judge(vec![
            Arc::new(FailDog),
            Arc::new(FixedDog {
                name: "survivor".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        assert_eq!(verdict.dog_id, "survivor");
    }

    #[tokio::test]
    async fn all_dogs_fail_returns_error() {
        let judge = test_judge(vec![Arc::new(FailDog)]);
        let result = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn no_dogs_returns_error() {
        let judge = test_judge(vec![]);
        let result = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn residual_detection_flags_high_disagreement() {
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "optimist".into(),
                scores: AxiomScores {
                    fidelity: 0.9,
                    phi: 0.9,
                    verify: 0.9,
                    culture: 0.9,
                    burn: 0.9,
                    sovereignty: 0.9,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "pessimist".into(),
                scores: AxiomScores {
                    fidelity: 0.1,
                    phi: 0.1,
                    verify: 0.1,
                    culture: 0.1,
                    burn: 0.1,
                    sovereignty: 0.1,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        assert!(
            verdict.anomaly_detected,
            "Dogs disagreeing 0.9 vs 0.1 should trigger anomaly"
        );
        assert!(verdict.max_disagreement > PHI_INV2);
        assert_eq!(verdict.dog_scores.len(), 2);
        assert!(verdict.anomaly_axiom.is_some());
    }

    #[tokio::test]
    async fn no_anomaly_when_dogs_agree() {
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "a".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "b".into(),
                scores: AxiomScores {
                    fidelity: 0.52,
                    phi: 0.48,
                    verify: 0.51,
                    culture: 0.49,
                    burn: 0.51,
                    sovereignty: 0.50,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        assert!(
            !verdict.anomaly_detected,
            "Similar scores should not trigger anomaly"
        );
        assert!(verdict.anomaly_axiom.is_none());
    }

    #[tokio::test]
    async fn filter_selects_specific_dogs() {
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "alpha".into(),
                scores: AxiomScores {
                    fidelity: 0.9,
                    phi: 0.9,
                    verify: 0.9,
                    culture: 0.9,
                    burn: 0.9,
                    sovereignty: 0.9,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "beta".into(),
                scores: AxiomScores {
                    fidelity: 0.1,
                    phi: 0.1,
                    verify: 0.1,
                    culture: 0.1,
                    burn: 0.1,
                    sovereignty: 0.1,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let filter = vec!["alpha".to_string()];
        let verdict = judge
            .evaluate(
                &test_stimulus(),
                Some(&filter),
                &test_metrics(),
                SlotPriority::User,
            )
            .await
            .unwrap();
        assert_eq!(verdict.dog_id, "alpha");
        assert_eq!(verdict.dog_scores.len(), 1);
    }

    #[tokio::test]
    async fn filter_with_unknown_id_returns_error() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "alpha".into(),
            scores: AxiomScores::default(),
        })]);

        let filter = vec!["nonexistent".to_string()];
        let result = judge
            .evaluate(
                &test_stimulus(),
                Some(&filter),
                &test_metrics(),
                SlotPriority::User,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stimulus_summary_truncated_at_100_chars() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "t".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let long_stimulus = Stimulus {
            content: "a".repeat(500),
            context: None,
            domain: None,
            request_id: None,
        };
        let verdict = judge
            .evaluate(&long_stimulus, None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        assert_eq!(verdict.stimulus_summary.len(), 300);
    }

    #[tokio::test]
    async fn circuit_breaker_skips_open_dog() {
        let judge = test_judge(vec![
            Arc::new(FailDog),
            Arc::new(FixedDog {
                name: "healthy".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        // Trip the circuit breaker on FailDog: 3 failures → Open
        for _ in 0..3 {
            let _ = judge
                .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
                .await;
        }

        // Now FailDog's circuit should be open — only healthy responds
        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        assert_eq!(
            verdict.dog_id, "healthy",
            "Open circuit should skip FailDog"
        );
        assert_eq!(verdict.dog_scores.len(), 1);
    }

    #[tokio::test]
    async fn median_reasoning_with_three_dogs() {
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "low".into(),
                scores: AxiomScores {
                    fidelity: 0.2,
                    phi: 0.2,
                    verify: 0.2,
                    culture: 0.2,
                    burn: 0.2,
                    sovereignty: 0.2,
                    reasoning: AxiomReasoning {
                        fidelity: "low reasoning".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "mid".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning {
                        fidelity: "mid reasoning".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "high".into(),
                scores: AxiomScores {
                    fidelity: 0.8,
                    phi: 0.8,
                    verify: 0.8,
                    culture: 0.8,
                    burn: 0.8,
                    sovereignty: 0.8,
                    reasoning: AxiomReasoning {
                        fidelity: "high reasoning".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        // Median of 3 sorted by Q → picks index 1 (mid)
        assert_eq!(verdict.reasoning.fidelity, "mid reasoning");
    }

    #[tokio::test]
    async fn dog_health_reports_all_dogs() {
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "a".into(),
                scores: AxiomScores::default(),
            }),
            Arc::new(FixedDog {
                name: "b".into(),
                scores: AxiomScores::default(),
            }),
        ]);

        let health = judge.dog_health();
        assert_eq!(health.len(), 2);
        assert_eq!(health[0].0, "a");
        assert_eq!(health[0].1, "closed"); // circuit starts closed
        assert_eq!(health[0].2, 0); // zero failures
    }

    #[tokio::test]
    async fn dog_ids_returns_all_names() {
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "x".into(),
                scores: AxiomScores::default(),
            }),
            Arc::new(FixedDog {
                name: "y".into(),
                scores: AxiomScores::default(),
            }),
        ]);
        let ids = judge.dog_ids();
        assert_eq!(ids, vec!["x", "y"]);
    }

    #[tokio::test]
    async fn verdict_has_valid_secure_id_and_timestamp() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "t".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        // Secure ID format: 32 chars hex
        assert_eq!(verdict.id.len(), 32);
        assert!(verdict.id.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(verdict.timestamp > 0);
    }

    fn make_dog_score(id: &str, val: f64) -> DogScore {
        DogScore {
            dog_id: id.into(),
            fidelity: val,
            phi: val,
            verify: val,
            culture: val,
            burn: val,
            sovereignty: val,
            raw_fidelity: val,
            raw_phi: val,
            raw_verify: val,
            raw_culture: val,
            raw_burn: val,
            raw_sovereignty: val,
            ..Default::default()
        }
    }

    #[test]
    fn detect_residuals_ignores_nan_scores() {
        let mut nan_dog = make_dog_score("nan-dog", 0.5);
        nan_dog.fidelity = f64::NAN;
        let normal = make_dog_score("normal", 0.5);
        let (spread, axiom) = math::detect_residuals(&[nan_dog, normal]);
        // NaN filtered → only 1 finite fidelity value → spread 0 on fidelity
        // Other axioms: both 0.5 → spread 0
        assert!(spread < 0.01, "NaN should not inflate spread, got {spread}");
        assert!(axiom.is_none());
    }

    // ── hash chain integrity ─────────────────────────────

    #[tokio::test]
    async fn verdict_chain_links_hashes() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "chain".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let v1 = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        let v2 = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();

        // v1 has no prev (first in chain)
        assert!(v1.prev_hash.is_none());
        assert!(v1.integrity_hash.is_some());

        // v2's prev_hash == v1's integrity_hash
        assert_eq!(v2.prev_hash, v1.integrity_hash);
        assert!(v2.integrity_hash.is_some());
        assert_ne!(v1.integrity_hash, v2.integrity_hash);
    }

    #[test]
    fn seed_chain_sets_initial_hash() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "s".into(),
            scores: AxiomScores::default(),
        })]);
        judge.seed_chain(Some("abc123".into()));
        let lock = judge.last_hash.lock().unwrap();
        assert_eq!(*lock, Some("abc123".into()));
    }

    // ── verdict integrity verification ──────────────────────

    #[tokio::test]
    async fn verify_integrity_passes_for_valid_verdict() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "v".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        assert!(
            verify_verdict_integrity(&verdict),
            "freshly created verdict must pass integrity check"
        );
    }

    #[tokio::test]
    async fn verify_integrity_fails_on_tampered_score() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "v".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let mut verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        // Tamper: modify a score after hash was computed
        verdict.q_score.fidelity = 0.999;
        assert!(
            !verify_verdict_integrity(&verdict),
            "tampered score must fail integrity check"
        );
    }

    #[tokio::test]
    async fn verify_integrity_fails_on_tampered_summary() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "v".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let mut verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        verdict.stimulus_summary = "TAMPERED".into();
        assert!(
            !verify_verdict_integrity(&verdict),
            "tampered stimulus must fail integrity check"
        );
    }

    #[test]
    fn verify_integrity_fails_when_hash_missing() {
        let verdict = Verdict {
            id: "test".into(),
            domain: "test".into(),
            kind: VerdictKind::Wag,
            q_score: QScore {
                total: 0.5,
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
            },
            reasoning: AxiomReasoning::default(),
            dog_id: "test".into(),
            stimulus_summary: "test".into(),
            timestamp: 1_767_225_600_000,
            dog_scores: vec![],
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            voter_count: 1,
            failed_dogs: vec![],
            failed_dog_errors: Default::default(),
            failed_dog_error_kinds: Default::default(),
            excluded_dogs: vec![],
            target: None,
            integrity_hash: None, // no hash
            prev_hash: None,
        };
        assert!(
            !verify_verdict_integrity(&verdict),
            "verdict without hash must fail verification"
        );
    }

    #[tokio::test]
    async fn verify_integrity_across_chain() {
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "chain".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let v1 = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();
        let v2 = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();

        // Both verdicts in chain verify independently
        assert!(verify_verdict_integrity(&v1));
        assert!(verify_verdict_integrity(&v2));
        // Chain linkage holds
        assert_eq!(v2.prev_hash, v1.integrity_hash);
    }

    #[tokio::test]
    async fn quality_degraded_dog_is_skipped() {
        let good_dog = FixedDog {
            name: "good".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        };
        let bad_dog = FixedDog {
            name: "bad".into(),
            scores: AxiomScores {
                fidelity: 0.1,
                phi: 0.1,
                verify: 0.1,
                culture: 0.1,
                burn: 0.1,
                sovereignty: 0.1,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        };

        let mut organ = crate::organ::InferenceOrgan::boot_empty();
        let handle = organ.register_backend(crate::organ::registry::Backend {
            id: crate::organ::registry::BackendId("bad".into()),
            declared: crate::organ::registry::DeclaredCapabilities::default(),
            measured: crate::organ::registry::MeasuredCapabilities::default(),
            health: crate::organ::registry::BackendHealth::Degraded {
                reason: "test quality gate".into(),
                since: std::time::Instant::now(),
            },
        });

        // K22: total_calls must be > 0, otherwise the quality gate is bypassed
        // (0 calls = no baseline to judge, so the dog gets a free pass).
        handle.record_outcome(crate::domain::dog_health::ScoreOutcome::Failure(
            crate::domain::dog_health::ScoreFailureKind::Timeout,
        ));

        let judge = test_judge(vec![Arc::new(good_dog), Arc::new(bad_dog)])
            .with_organ_handles(vec![None, Some(handle)]);

        let metrics = Arc::new(test_metrics());
        let verdict = judge
            .evaluate(&test_stimulus(), None, &metrics, SlotPriority::User)
            .await
            .unwrap();

        // bad dog is quality-degraded + has call history → should be skipped
        assert_eq!(verdict.dog_scores.len(), 1);
        assert_eq!(verdict.dog_scores[0].dog_id, "good");
    }

    // ── EPOCHÉ integration tests ─────────────────────────────

    #[tokio::test]
    async fn epoche_emitted_on_high_disagreement() {
        // Two Dogs maximally opposed on BURN:
        //   dog_a: burn=0.05 (floor after phi_bound), dog_b: burn=0.6
        //   max_disagreement on burn ≈ 0.55 (after phi_bound: |0.05 - 0.618| capped ≈ 0.55)
        // Seed window with 100 low values (0.01..=0.10) so P95 ≈ 0.095.
        // 0.55 > 0.095 with 2 voters → EPOCHÉ.
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "dog-a".into(),
                scores: AxiomScores {
                    fidelity: 0.4,
                    phi: 0.4,
                    verify: 0.4,
                    culture: 0.4,
                    burn: 0.05,
                    sovereignty: 0.4,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "dog-b".into(),
                scores: AxiomScores {
                    fidelity: 0.4,
                    phi: 0.4,
                    verify: 0.4,
                    culture: 0.4,
                    burn: 0.6,
                    sovereignty: 0.4,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        // Seed window with 100 low values so P95 is low (~0.095).
        let low_disagreements: Vec<f64> = (1..=100).map(|i| i as f64 * 0.001).collect();
        judge.seed_epoche_window(&low_disagreements);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();

        assert_eq!(
            verdict.kind,
            VerdictKind::Epoche,
            "High disagreement above P95 threshold must produce EPOCHÉ verdict"
        );
        // Q-score still computed for diagnostics
        assert!(
            verdict.q_score.total > 0.0,
            "Q-score must be non-zero even on EPOCHÉ"
        );
    }

    #[tokio::test]
    async fn normal_verdict_when_below_threshold() {
        // Two Dogs with close BURN scores → disagreement ≈ 0.05.
        // Seed window with moderate values (0.3) so P95 ≈ 0.3.
        // 0.05 < 0.3 → normal verdict (not Epoche).
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "dog-c".into(),
                scores: AxiomScores {
                    fidelity: 0.45,
                    phi: 0.45,
                    verify: 0.45,
                    culture: 0.45,
                    burn: 0.40,
                    sovereignty: 0.45,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "dog-d".into(),
                scores: AxiomScores {
                    fidelity: 0.45,
                    phi: 0.45,
                    verify: 0.45,
                    culture: 0.45,
                    burn: 0.45,
                    sovereignty: 0.45,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        // Seed window with 100 moderate values (0.3) so P95 ≈ 0.3.
        let moderate_disagreements: Vec<f64> = (1..=100).map(|_| 0.3).collect();
        judge.seed_epoche_window(&moderate_disagreements);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();

        assert_ne!(
            verdict.kind,
            VerdictKind::Epoche,
            "Low disagreement below P95 threshold must not produce EPOCHÉ verdict"
        );
    }

    #[tokio::test]
    async fn epoche_not_triggered_on_single_dog() {
        // A single Dog can never disagree with itself — EPOCHÉ requires voter_count >= 2.
        let judge = test_judge(vec![Arc::new(FixedDog {
            name: "solo".into(),
            scores: AxiomScores {
                fidelity: 0.9,
                phi: 0.9,
                verify: 0.9,
                culture: 0.9,
                burn: 0.9,
                sovereignty: 0.9,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        // Seed with a tiny threshold so any non-zero disagreement would normally suspend.
        let tiny_disagreements: Vec<f64> = (1..=100).map(|i| i as f64 * 0.00001).collect();
        judge.seed_epoche_window(&tiny_disagreements);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics(), SlotPriority::User)
            .await
            .unwrap();

        assert_ne!(
            verdict.kind,
            VerdictKind::Epoche,
            "Single Dog must never produce EPOCHÉ (no disagreement possible)"
        );
    }

    #[tokio::test]
    async fn without_dog_propagates_epoche_window() {
        // Verify that Judge::without_dog propagates the EPOCHÉ window.
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "alpha".into(),
                scores: AxiomScores::default(),
            }),
            Arc::new(FixedDog {
                name: "beta".into(),
                scores: AxiomScores::default(),
            }),
        ]);

        // Seed with 50 values so window is not empty after propagation.
        let values: Vec<f64> = (1..=50).map(|i| i as f64 * 0.01).collect();
        judge.seed_epoche_window(&values);

        // Remove one Dog — window must propagate.
        let reduced = Judge::without_dog(&judge, "beta").expect("beta exists");

        // Probe: threshold exists iff window has >= MIN_SAMPLES (10).
        // We seeded 50 values — threshold must exist after propagation.
        let threshold_exists = reduced
            .epoche_threshold
            .lock()
            .unwrap()
            .threshold()
            .is_some();
        assert!(
            threshold_exists,
            "EPOCHÉ window must propagate through Judge::without_dog (window must have >= MIN_SAMPLES)"
        );
    }

    #[tokio::test]
    async fn filter_excludes_deterministic_dog_when_not_requested() {
        // Regression test for forced consensus removal (commit d0dd481).
        // Verifies that deterministic-dog is NOT forced-included when filtering dogs.
        // Previously, the filter logic was: d.id() == "deterministic-dog" || ids.iter().any(...)
        // Now it should only include dogs explicitly requested in the filter.
        let judge = test_judge(vec![
            Arc::new(FixedDog {
                name: "deterministic-dog".into(),
                scores: AxiomScores {
                    fidelity: 0.9,
                    phi: 0.9,
                    verify: 0.9,
                    culture: 0.9,
                    burn: 0.9,
                    sovereignty: 0.9,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "test-dog-1".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Arc::new(FixedDog {
                name: "test-dog-2".into(),
                scores: AxiomScores {
                    fidelity: 0.4,
                    phi: 0.4,
                    verify: 0.4,
                    culture: 0.4,
                    burn: 0.4,
                    sovereignty: 0.4,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        // Filter requesting ONLY test-dog-1 and test-dog-2 (explicitly exclude deterministic-dog)
        let filter = vec!["test-dog-1".to_string(), "test-dog-2".to_string()];
        let verdict = judge
            .evaluate(
                &test_stimulus(),
                Some(&filter),
                &test_metrics(),
                SlotPriority::User,
            )
            .await
            .unwrap();

        // Assert: only 2 dogs scored (not 3)
        assert_eq!(
            verdict.dog_scores.len(),
            2,
            "Expected 2 dogs to be evaluated, but got {}",
            verdict.dog_scores.len()
        );

        // Assert: deterministic-dog is NOT in the result
        for dog_score in &verdict.dog_scores {
            assert_ne!(
                dog_score.dog_id, "deterministic-dog",
                "deterministic-dog should not be force-included when not explicitly requested"
            );
        }

        // Assert: the returned dogs are the ones we requested
        let dog_ids: Vec<&str> = verdict
            .dog_scores
            .iter()
            .map(|s| s.dog_id.as_str())
            .collect();
        assert!(
            dog_ids.contains(&"test-dog-1"),
            "test-dog-1 should be in the result"
        );
        assert!(
            dog_ids.contains(&"test-dog-2"),
            "test-dog-2 should be in the result"
        );
    }
}
