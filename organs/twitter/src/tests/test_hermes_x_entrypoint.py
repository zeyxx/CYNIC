import sys
import unittest

from organs.twitter.src.entrypoints import hermes_x


class HermesXEntrypointTests(unittest.TestCase):
    def test_resolves_known_component_to_existing_legacy_script(self):
        component = hermes_x.resolve_component("x-ingest")

        self.assertTrue(component.script.exists())
        self.assertTrue(str(component.script).endswith("organs/twitter/src/entrypoints/x_ingest.py"))

    def test_resolves_remaining_special_components(self):
        for name in ("browser-hub", "search-executor", "x-recovery", "x-login"):
            with self.subTest(name=name):
                self.assertTrue(hermes_x.resolve_component(name).script.exists())

    def test_search_executor_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("search-executor")

        self.assertIn("organs/twitter/src/adapters/search_executor.py", str(component.script))

    def test_browser_hub_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("browser-hub")

        self.assertIn("organs/twitter/src/adapters/browser_hub.py", str(component.script))

    def test_search_generator_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("search-generator")

        self.assertIn("organs/twitter/src/entrypoints/search_generation.py", str(component.script))

    def test_curation_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("curation")

        self.assertIn("organs/twitter/src/entrypoints/curate_domain_signals.py", str(component.script))

    def test_navigator_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("navigator")

        self.assertIn("organs/twitter/src/entrypoints/organic_navigator.py", str(component.script))

    def test_k15_consumer_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("k15-consumer")

        self.assertIn("organs/twitter/src/entrypoints/k15_observation_consumer.py", str(component.script))
        self.assertIn(
            hermes_x.PROJECT_ROOT / "services" / "cynic-python" / "heuristics",
            component.extra_paths,
        )

    def test_behavior_logger_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("behavior-logger")

        self.assertIn("organs/twitter/src/entrypoints/behavior_logger.py", str(component.script))

    def test_task_runner_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("task-runner")

        self.assertIn("organs/twitter/src/entrypoints/hermes_task_runner.py", str(component.script))

    def test_gemini_briefing_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("gemini-briefing")

        self.assertIn("organs/twitter/src/entrypoints/gemini_briefing_consumer.py", str(component.script))

    def test_feedback_loop_resolves_to_organ_twitter(self):
        component = hermes_x.resolve_component("feedback-loop")

        self.assertIn("organs/twitter/src/entrypoints/gemini_learn_from_verdicts.py", str(component.script))

    def test_x_auth_components_resolve_to_organ_twitter(self):
        login = hermes_x.resolve_component("x-login")
        recovery = hermes_x.resolve_component("x-recovery")

        self.assertIn("organs/twitter/src/entrypoints/hermes_x_login.py", str(login.script))
        self.assertIn("organs/twitter/src/entrypoints/hermes_x_recovery.py", str(recovery.script))

    def test_x_poster_and_k15_emitter_resolve_to_organ_twitter(self):
        poster = hermes_x.resolve_component("x-poster")
        emitter = hermes_x.resolve_component("k15-emitter")

        self.assertIn("organs/twitter/src/entrypoints/x_poster.py", str(poster.script))
        self.assertIn("organs/twitter/src/adapters/k15_emitter.py", str(emitter.script))

    def test_rejects_unknown_component(self):
        with self.assertRaisesRegex(ValueError, "unknown Hermes X component"):
            hermes_x.resolve_component("not-real")

    def test_prepare_legacy_imports_does_not_add_legacy_script_paths(self):
        component = hermes_x.resolve_component("feedback-loop")
        before = list(sys.path)
        try:
            hermes_x.prepare_legacy_imports(component)
            self.assertIn(str(hermes_x.PROJECT_ROOT), sys.path)
            self.assertIn(str(hermes_x.ORGAN_TWITTER_ADAPTERS), sys.path)
            self.assertNotIn(str(hermes_x.LEGACY_ROOT / "core"), sys.path)
            self.assertNotIn(str(hermes_x.LEGACY_ROOT / "analysis"), sys.path)
            self.assertNotIn(str(hermes_x.LEGACY_ROOT / "consumers"), sys.path)
        finally:
            sys.path[:] = before


if __name__ == "__main__":
    unittest.main()
