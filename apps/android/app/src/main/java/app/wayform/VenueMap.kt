package app.wayform

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.min

private fun JSONArray.points() = (0 until length()).map { getJSONArray(it).let { p -> Offset(p.getDouble(0).toFloat(), p.getDouble(1).toFloat()) } }

@Composable
fun VenueMap(catalog: VenueCatalog, route: JSONObject?, floorId: String, modifier: Modifier = Modifier) {
    val floor = remember(catalog, floorId) { catalog.raw.getJSONArray("floors").objects().first { it.getString("id") == floorId } }
    val outline = remember(floor) { floor.getJSONArray("outline").points() }
    val spaces = remember(catalog, floorId) { catalog.raw.getJSONArray("spaces").objects().filter { it.getString("floorId") == floorId }.map { it to it.getJSONArray("polygon").points() } }
    val path = remember(route) { route?.optJSONArray("path")?.objects().orEmpty() }
    var zoom by remember(catalog.id, floorId) { mutableFloatStateOf(1f) }
    var pan by remember(catalog.id, floorId) { mutableStateOf(Offset.Zero) }
    val all = outline + spaces.flatMap { it.second }
    val minX = all.minOf { it.x }; val minY = all.minOf { it.y }
    val width = (all.maxOf { it.x } - minX).coerceAtLeast(1f)
    val height = (all.maxOf { it.y } - minY).coerceAtLeast(1f)
    Column(modifier) {
        Canvas(Modifier.fillMaxWidth().height(300.dp).clipToBounds().background(Color(0xFFE9EEE6)).semantics { contentDescription = "Map of ${floor.getString("name")}. Route instructions are available below. Pinch to zoom." }.pointerInput(catalog.id, floorId) {
            detectTransformGestures { _, movement, factor, _ -> zoom = (zoom * factor).coerceIn(1f, 6f); pan += movement }
        }) {
            val scale = min((size.width - 36.dp.toPx()) / width, (size.height - 36.dp.toPx()) / height) * zoom
            val offset = Offset((size.width - width * scale) / 2f, (size.height - height * scale) / 2f) + pan
            fun project(p: Offset) = Offset((p.x - minX) * scale, (p.y - minY) * scale) + offset
            fun polygon(points: List<Offset>) = Path().apply {
                points.forEachIndexed { index, p -> val q = project(p); if (index == 0) moveTo(q.x,q.y) else lineTo(q.x,q.y) }; close()
            }
            drawPath(polygon(outline), Color.White)
            spaces.forEach { (space, points) ->
                drawPath(polygon(points), if (space.getString("type") == "corridor") Color(0xFFF6F8F2) else Color(0xFFD6DFD3))
                drawPath(polygon(points), Color(0xFFADBCA9), style = Stroke(1.dp.toPx()))
            }
            val clearance = route?.optJSONObject("displayClearance")
            if (clearance?.optString("status") == "checked" && clearance.optBoolean("centerlineValid") && clearance.getJSONArray("issues").length() == 0) {
                // The stroke stays inside the core's checked physical envelope at every zoom.
                val stroke = (clearance.getDouble("widthMeters").toFloat() * scale).coerceAtMost(8.dp.toPx())
                path.zipWithNext().filter { (a,b) -> a.getString("floor") == floorId && b.getString("floor") == floorId }.forEach { (a,b) ->
                    fun point(n: JSONObject) = project(Offset(n.getDouble("x").toFloat(),n.getDouble("y").toFloat()))
                    drawLine(Forest, point(a), point(b), strokeWidth = stroke, cap = StrokeCap.Round)
                }
            }
            listOf(path.firstOrNull() to Forest, path.lastOrNull() to Color(0xFF8B4D27)).filter { it.first?.getString("floor") == floorId }.forEach { (maybeNode, color) ->
                val node = requireNotNull(maybeNode)
                val p = project(Offset(node.getDouble("x").toFloat(),node.getDouble("y").toFloat()))
                drawCircle(Color.White, 9.dp.toPx(), p)
                drawCircle(color, 6.dp.toPx(), p)
            }
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("Plan view · north may differ", style = MaterialTheme.typography.labelSmall, modifier = Modifier.padding(top = 14.dp))
            TextButton(onClick = { zoom = 1f; pan = Offset.Zero }) { Text("Reset map") }
        }
    }
}
