export interface CynicConfig {
    url: string;
    namespace: string;
    database: string;
    token?: string;
}
export interface ObservationEvent {
    id: string;
    timestamp: string;
    source_organ: string;
    event_type: string;
    payload: Record<string, any>;
    cryptographic_hash: string;
    prev_hash?: string;
}
export type LiveQueryCallback<T> = (action: 'CREATE' | 'UPDATE' | 'DELETE' | 'CLOSE', result: T) => void;
