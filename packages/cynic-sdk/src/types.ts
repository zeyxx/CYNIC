export interface CynicConfig {
    url: string;      // e.g., ws://127.0.0.1:8000/rpc
    namespace: string;
    database: string;
    token?: string;   // Auth token for SurrealDB
}

export interface ObservationEvent {
    id: string;
    timestamp: number; // Unix Epoch in milliseconds (L1 Truth)
    source_organ: string;
    event_type: string;
    payload: Record<string, any>;
    cryptographic_hash: string;
    prev_hash?: string;
}

export type LiveQueryCallback<T> = (action: 'CREATE' | 'UPDATE' | 'DELETE' | 'CLOSE', result: T) => void;
