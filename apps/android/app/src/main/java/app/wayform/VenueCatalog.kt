package app.wayform

import org.json.JSONArray
import org.json.JSONObject

fun JSONArray.objects(): List<JSONObject> = (0 until length()).map { getJSONObject(it) }

data class Destination(val id: String, val name: String, val floorId: String, val floorName: String, val category: String, val aliases: List<String>) {
    fun matches(query: String): Boolean = (listOf(name, category, floorName) + aliases).any { it.contains(query.trim(), ignoreCase = true) }
}
data class VenuePortal(val floorId: String, val x: Float, val y: Float, val width: Float, val rooms: Set<String>) {
    companion object {
        fun fromJson(portal: JSONObject): VenuePortal {
            val position = portal.getJSONArray("position")
            val connects = portal.getJSONArray("connects")
            return VenuePortal(
                portal.getString("floorId"), position.getDouble(0).toFloat(), position.getDouble(1).toFloat(),
                portal.getDouble("width").toFloat(), (0 until connects.length()).map(connects::getString).toSet(),
            )
        }
    }
}

data class VenueCatalog(val raw: JSONObject, val liftIds: List<String> = emptyList(), val portals: List<VenuePortal> = emptyList()) {
    val id: String = raw.getJSONObject("building").getString("id")
    val name: String = raw.getJSONObject("building").getString("name")
    val hash: String = raw.getString("packageHash")
    val startId: String = raw.getString("defaultStartNodeId")
    val destinations: List<Destination> = raw.getJSONArray("destinations").objects().map { node ->
        val poi = node.getJSONObject("poi")
        val aliases = poi.getJSONArray("aliases")
        Destination(node.getString("id"), poi.getString("name"), node.getString("floor"), node.getString("floorName"), poi.getString("category"), (0 until aliases.length()).map { aliases.getString(it) })
    }.sortedBy { it.name.lowercase() }
}
