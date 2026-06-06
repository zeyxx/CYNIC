package so.cynic.kyon

import android.app.Application
import androidx.room.Room
import so.cynic.kyon.core.storage.KyonDatabase

class KyonApp : Application() {
    lateinit var database: KyonDatabase
        private set

    override fun onCreate() {
        super.onCreate()
        database = Room.databaseBuilder(
            applicationContext,
            KyonDatabase::class.java,
            "kyon.db",
        ).build()
    }
}
