"""
CYNIC Data Schema — SurrealDB (MLOps/Data Engineering).

Defines the tables and indexes for the Organism's memory:
- dog_souls: Identities and weights of the Dogs.
- action_proposals: Pending and executed actions.
- judgments: Full history of consensus results.
- q_table: State-Action reinforcement learning values.
"""

def get_schema_queries():
    return [
        "DEFINE TABLE dog_souls SCHEMAFULL;",
        "DEFINE FIELD sefirot ON TABLE dog_souls TYPE string;",
        "DEFINE FIELD weights ON TABLE dog_souls TYPE object;",
        
        "DEFINE TABLE action_proposals SCHEMAFULL;",
        "DEFINE FIELD status ON TABLE action_proposals TYPE string ASSERT $value IN ['PENDING', 'APPROVED', 'EXECUTED', 'FAILED'];",
        
        "DEFINE TABLE q_table SCHEMAFULL;",
        "DEFINE INDEX idx_state_action ON TABLE q_table FIELDS state_key, action UNIQUE;",
        
        "DEFINE TABLE judgments SCHEMAFULL;",
        "DEFINE FIELD q_score ON TABLE judgments TYPE number;",
    ]

if __name__ == "__main__":
    print("Generating SurrealDB Schema...")
    for q in get_schema_queries():
        print(q)
