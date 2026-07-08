package so.cynic.kyon.ports

interface PeerHasher {
    fun hash(peer: String): String
}
