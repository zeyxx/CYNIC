import { CynicConfig, ObservationEvent, LiveQueryCallback } from './types';
export declare class CynicClient {
    private db;
    private config;
    private liveQueryIds;
    constructor(config: CynicConfig);
    /**
     * Connects to the CYNIC SurrealDB instance via WebSocket.
     */
    connect(): Promise<void>;
    /**
     * Disconnects and kills all active streams to prevent memory leaks.
     */
    disconnect(): Promise<void>;
    /**
     * Replaces toxic polling by subscribing directly to the CYNIC State Engine.
     * Streams every new block (Proof-of-History observation) instantly over the WebSocket.
     */
    streamUnipersonalBlockchain(callback: LiveQueryCallback<ObservationEvent>): Promise<string>;
    /**
     * Example of a specific domain logic abstracted from raw SurrealQL.
     * Computes the E-Score of a given wallet directly from the graph relationships.
     */
    getEScore(walletAddress: string): Promise<number>;
}
