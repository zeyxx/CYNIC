"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.CynicClient = void 0;
// @ts-ignore
const surrealdb_js_1 = __importDefault(require("surrealdb.js"));
class CynicClient {
    db;
    config;
    liveQueryIds = new Set();
    constructor(config) {
        this.config = config;
        this.db = new surrealdb_js_1.default();
    }
    /**
     * Connects to the CYNIC SurrealDB instance via WebSocket.
     */
    async connect() {
        await this.db.connect(this.config.url);
        await this.db.use({ namespace: this.config.namespace, database: this.config.database });
        if (this.config.token) {
            await this.db.authenticate(this.config.token);
        }
    }
    /**
     * Disconnects and kills all active streams to prevent memory leaks.
     */
    async disconnect() {
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
    async streamUnipersonalBlockchain(callback) {
        // Issue a LIVE SELECT to SurrealDB via the WebSocket
        const queryUuid = await this.db.live('observations', (action, result) => {
            callback(action, result);
        });
        this.liveQueryIds.add(queryUuid);
        return queryUuid;
    }
    /**
     * Example of a specific domain logic abstracted from raw SurrealQL.
     * Computes the E-Score of a given wallet directly from the graph relationships.
     */
    async getEScore(walletAddress) {
        const result = await this.db.query('SELECT math::sum(->scored_by.score) as total_score FROM type::thing("wallet", $walletAddress)', { walletAddress });
        return result[0]?.[0]?.total_score || 0;
    }
}
exports.CynicClient = CynicClient;
