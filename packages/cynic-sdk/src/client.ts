// @ts-ignore
import Surreal from 'surrealdb.js';
import { CynicConfig, ObservationEvent, LiveQueryCallback } from './types';

export class CynicClient {
    private db: any;
    private config: CynicConfig;
    private liveQueryIds: Set<string> = new Set();

    constructor(config: CynicConfig) {
        this.config = config;
        this.db = new Surreal();
    }

    /**
     * Connects to the CYNIC SurrealDB instance via WebSocket.
     */
    public async connect(): Promise<void> {
        await this.db.connect(this.config.url);
        await this.db.use({ namespace: this.config.namespace, database: this.config.database });
        
        if (this.config.token) {
            await this.db.authenticate(this.config.token);
        }
    }

    /**
     * Disconnects and kills all active streams to prevent memory leaks.
     */
    public async disconnect(): Promise<void> {
        for (const queryId of this.liveQueryIds) {
            await this.db.kill(queryId);
        }
        this.liveQueryIds.clear();
        await this.db.close();
    }

    /**
     * Replaces toxic polling by subscribing directly to the CYNIC State Engine.
     * Streams every new block (Proof-of-History observation) instantly over the WebSocket.
     */
    public async streamUnipersonalBlockchain(
        callback: LiveQueryCallback<ObservationEvent>
    ): Promise<string> {
        // Issue a LIVE SELECT to SurrealDB via the WebSocket
        const queryUuid = await this.db.live('observations', (action: any, result: any) => {
            callback(action, result as ObservationEvent);
        });
        
        this.liveQueryIds.add(queryUuid);
        return queryUuid;
    }

    /**
     * Example of a specific domain logic abstracted from raw SurrealQL.
     * Computes the E-Score of a given wallet directly from the graph relationships.
     */
    public async getEScore(walletAddress: string): Promise<number> {
        const result = await this.db.query(
            'SELECT math::sum(->scored_by.score) as total_score FROM type::thing("wallet", $walletAddress)',
            { walletAddress }
        );
        return result[0]?.[0]?.total_score || 0;
    }
}
