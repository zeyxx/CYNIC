try:
    from .test_validate_repo_topology_git_membrane_a import GitMembraneClosureSummaryTests
    from .test_validate_repo_topology_git_membrane_b import GitMembraneClosurePlanTests
except ImportError:
    from test_validate_repo_topology_git_membrane_a import GitMembraneClosureSummaryTests
    from test_validate_repo_topology_git_membrane_b import GitMembraneClosurePlanTests
