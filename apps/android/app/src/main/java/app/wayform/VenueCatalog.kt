package app.wayform

import org.json.JSONArray
import org.json.JSONObject

fun JSONArray.objects(): List<JSONObject> = (0 until length()).map { getJSONObject(it) }

data class Destination(val id: String, val name: String, val floorId: String, val floorName: String, val category: String, val aliases: List<String>) {
    fun matches(query: String): Boolean = (listOf(name, category, floorName) + aliases).any { it.contains(query.trim(), ignoreCase = true) }
}
data class VenueCatalog(val raw: JSONObject, val liftIds: List<String> = emptyList()) {
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
