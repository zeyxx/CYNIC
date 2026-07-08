package so.cynic.kyon.adapters.security

import so.cynic.kyon.ports.PeerHasher
import java.security.MessageDigest
import java.util.Locale

class Sha256PeerHasher : PeerHasher {
    override fun hash(peer: String): String {
        val normalized = peer.filter { !it.isWhitespace() }.lowercase(Locale.US)
        val input = normalized.ifBlank { "unknown" }
        val digest = MessageDigest.getInstance("SHA-256").digest(input.toByteArray())
        return digest.joinToString(separator = "") { "%02x".format(it) }.take(16)
    }
}
