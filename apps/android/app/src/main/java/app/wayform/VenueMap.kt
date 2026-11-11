package app.wayform

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
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
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import android.graphics.Paint
import android.graphics.RectF
import android.text.TextPaint
import android.text.TextUtils
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.min

private fun JSONArray.points() = (0 until length()).map { getJSONArray(it).let { p -> Offset(p.getDouble(0).toFloat(), p.getDouble(1).toFloat()) } }

private fun roomLabel(name: String, paint: Paint, width: Float): List<String> {
    val lines = mutableListOf<String>()
    var line = ""
    for (word in name.split(' ')) {
        if (paint.measureText(word) > width) return emptyList()
        val next = if (line.isEmpty()) word else "$line $word"
        if (paint.measureText(next) <= width) line = next
        else { lines.add(line); line = word }
    }
    if (line.isNotEmpty()) lines.add(line)
    return lines
}

@Composable
fun VenueMap(catalog: VenueCatalog, route: JSONObject?, floorId: String, modifier: Modifier = Modifier) {
    val floor = remember(catalog, floorId) { catalog.raw.getJSONArray("floors").objects().first { it.getString("id") == floorId } }
    val outline = remember(floor) { floor.getJSONArray("outline").points() }
    val spaces = remember(catalog, floorId) { catalog.raw.getJSONArray("spaces").objects().filter { it.getString("floorId") == floorId }.map { it to it.getJSONArray("polygon").points() } }
    val path = remember(route) { route?.optJSONArray("path")?.objects().orEmpty() }
    val occupiedSpaces = remember(path, spaces, floorId) {
        val nodes = path.filter { it.getString("floor") == floorId }
            .map { Offset(it.getDouble("x").toFloat(), it.getDouble("y").toFloat()) }
        spaces.mapNotNull { (space, points) ->
            val x = points.minOf { it.x }..points.maxOf { it.x }
            val y = points.minOf { it.y }..points.maxOf { it.y }
            space.getString("id").takeIf { nodes.any { point -> point.x in x && point.y in y } }
        }.toSet()
    }
    val floorNames = remember(catalog) { catalog.raw.getJSONArray("floors").objects().associate { it.getString("id") to it.getString("name").substringBefore(" · ") } }
    val labelPaint = remember { TextPaint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER } }
    val bubblePaint = remember { Paint(Paint.ANTI_ALIAS_FLAG) }
    var zoom by remember(catalog.id, floorId) { mutableFloatStateOf(1f) }
    var pan by remember(catalog.id, floorId) { mutableStateOf(Offset.Zero) }
    val all = outline + spaces.flatMap { it.second }
    val minX = all.minOf { it.x }; val minY = all.minOf { it.y }
    val width = (all.maxOf { it.x } - minX).coerceAtLeast(1f)
    val height = (all.maxOf { it.y } - minY).coerceAtLeast(1f)
    Column(modifier) {
        Canvas(Modifier.fillMaxWidth().height(218.dp).clipToBounds().background(Color(0xFFEAF0EB)).semantics { contentDescription = "Map of ${floor.getString("name")}. Pinch to zoom." }.pointerInput(catalog.id, floorId) {
            awaitEachGesture {
                awaitFirstDown()
                do {
                    val event = awaitPointerEvent()
                    if (event.changes.any { it.isConsumed }) break
                    if (event.changes.count { it.pressed } >= 2 || zoom > 1f) {
                        zoom = (zoom * event.calculateZoom()).coerceIn(1f, 6f)
                        pan = if (zoom == 1f) Offset.Zero else pan + event.calculatePan()
                        event.changes.forEach { it.consume() }
                    }
                } while (event.changes.any { it.pressed })
            }
        }) {
            val scale = min((size.width - 36.dp.toPx()) / width, (size.height - 36.dp.toPx()) / height) * zoom
            val offset = Offset((size.width - width * scale) / 2f, (size.height - height * scale) / 2f) + pan
            fun project(p: Offset) = Offset((p.x - minX) * scale, (p.y - minY) * scale) + offset
            fun polygon(points: List<Offset>) = Path().apply {
                points.forEachIndexed { index, p -> val q = project(p); if (index == 0) moveTo(q.x,q.y) else lineTo(q.x,q.y) }; close()
            }
            drawPath(polygon(outline), Color.White)
            spaces.forEach { (space, points) ->
                val shape = polygon(points)
                val fill = when (space.getString("type")) {
                    "corridor" -> Color.White
                    "lobby", "entrance" -> Color(0xFFD2E9DA)
                    "service" -> Color(0xFFE0E2EE)
                    else -> Color(0xFFE2E7DF)
                }
                drawPath(shape, fill)
                drawPath(shape, Color(0xFFB0C0B7), style = Stroke(1.dp.toPx()))
                val bounds = shape.getBounds()
                labelPaint.textSize = 11.sp.toPx()
                labelPaint.color = Muted.toArgb()
                if (space.getString("id") !in occupiedSpaces && (path.isEmpty() || space.getString("type") != "corridor")) {
                    val lines = roomLabel(space.getString("name"), labelPaint, bounds.width - 5.dp.toPx())
                    val lineHeight = labelPaint.descent() - labelPaint.ascent()
                    if (lines.size * lineHeight <= bounds.height - 5.dp.toPx()) drawIntoCanvas {
                        val firstBaseline = bounds.center.y - lines.size * lineHeight / 2 - labelPaint.ascent()
                        lines.forEachIndexed { index, line ->
                            it.nativeCanvas.drawText(line, bounds.center.x, firstBaseline + index * lineHeight, labelPaint)
                        }
                    }
                }
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
            path.forEachIndexed { index, node ->
                if (node.getString("floor") != floorId) return@forEachIndexed
                val nextFloor = path.getOrNull(index + 1)?.getString("floor")
                val label = when {
                    index == path.lastIndex -> "Destination"
                    index == 0 -> "Start"
                    nextFloor != null && nextFloor != floorId -> floorNames[nextFloor]?.let { "To $it" } ?: "Continue"
                    else -> return@forEachIndexed
                }
                val p = project(Offset(node.getDouble("x").toFloat(),node.getDouble("y").toFloat()))
                if (p.x !in 0f..size.width || p.y !in 0f..size.height) return@forEachIndexed
                val color = if (label == "Destination") Color(0xFF78492F) else Forest
                drawCircle(Color.White, 6.dp.toPx(), p)
                drawCircle(color, 4.dp.toPx(), p)
                labelPaint.textSize = 12.sp.toPx()
                labelPaint.color = Color.White.toArgb()
                val caption = TextUtils.ellipsize(label, labelPaint, size.width * 0.45f, TextUtils.TruncateAt.END).toString()
                val half = labelPaint.measureText(caption) / 2 + 7.dp.toPx()
                val x = p.x.coerceIn(half, size.width - half)
                val h = labelPaint.descent() - labelPaint.ascent() + 6.dp.toPx()
                val top = if (p.y > h + 8.dp.toPx()) p.y - h - 6.dp.toPx() else p.y + 6.dp.toPx()
                bubblePaint.color = color.toArgb()
                drawIntoCanvas {
                    it.nativeCanvas.drawRoundRect(RectF(x-half, top, x+half, top+h), 6.dp.toPx(), 6.dp.toPx(), bubblePaint)
                    it.nativeCanvas.drawText(caption, x, top + 3.dp.toPx() - labelPaint.ascent(), labelPaint)
                }
            }
        }
        if (zoom != 1f || pan != Offset.Zero) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
            TextButton(onClick = { zoom = 1f; pan = Offset.Zero }) { Text("Reset map") }
        }
    }
}
