package app.wayform

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import app.wayform.core.MobileVenue
import org.json.JSONObject

data class VenueEntry(val id: String, val name: String, val hash: String)

/** Verified bytes are committed atomically; active journeys are never persisted. */
class VenueStore(context: Context) : SQLiteOpenHelper(context, "venues.db", null, 1) {
    override fun onCreate(db: SQLiteDatabase) {
        db.execSQL("CREATE TABLE venues (id TEXT PRIMARY KEY, name TEXT NOT NULL, hash TEXT NOT NULL, package BLOB NOT NULL)")
    }
    override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) = Unit

    fun save(bytes: ByteArray): VenueEntry {
        // Leave room for row metadata in Android's cursor window on older devices.
        require(bytes.size <= 1024 * 1024) { "Venue packages must be 1 MiB or smaller." }
        val catalog = MobileVenue(bytes).use { JSONObject(it.catalog()) }
        val building = catalog.getJSONObject("building")
        val entry = VenueEntry(building.getString("id"), building.getString("name"), catalog.getString("packageHash"))
        val values = ContentValues().apply {
            put("id", entry.id); put("name", entry.name); put("hash", entry.hash); put("package", bytes)
        }
        writableDatabase.beginTransaction()
        try {
            writableDatabase.insertWithOnConflict("venues", null, values, SQLiteDatabase.CONFLICT_REPLACE).also {
                check(it != -1L) { "Could not save this venue." }
            }
            writableDatabase.setTransactionSuccessful()
        } finally { writableDatabase.endTransaction() }
        return entry
    }
    fun entries(): List<VenueEntry> = readableDatabase.rawQuery("SELECT id,name,hash FROM venues ORDER BY name", null).use { cursor ->
        buildList { while (cursor.moveToNext()) add(VenueEntry(cursor.getString(0), cursor.getString(1), cursor.getString(2))) }
    }
    fun bytes(id: String): ByteArray = readableDatabase.rawQuery("SELECT package FROM venues WHERE id=?", arrayOf(id)).use {
        check(it.moveToFirst()) { "This venue is no longer available offline." }
        it.getBlob(0)
    }
    fun seed(context: Context) {
        val present = entries().map { it.id }.toSet()
        context.assets.list("venues").orEmpty().filter { it.endsWith(".json") }.forEach { file ->
            if (file.removeSuffix(".json") !in present) context.assets.open("venues/$file").use { save(it.readBytes()) }
        }
    }
}
