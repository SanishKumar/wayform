package app.wayform

import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.text.TextPaint
import android.text.TextUtils
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.min

enum class MapViewMode { DEPTH, PLAN }

private val MapInk = Color(0xFF121916)
private val RouteLime = Color(0xFFCBFF78)
private val MapText = Color(0xFFDBE4DA)
private val MapShadow = Color(0xB3080B0A)

private fun JSONArray.mapPoint() = Offset(getDouble(0).toFloat(), getDouble(1).toFloat())

private fun JSONArray.points() = (0 until length()).map { getJSONArray(it).mapPoint() }

private data class MapArea(val name: String, val kind: String, val points: List<Offset>, val id: String = "")
private data class SiteBuilding(val name: String, val storeys: Int, val footprint: List<Offset>)
private data class SiteFeature(val kind: String, val position: Offset)
private data class MapNode(val id: String, val floor: String, val point: Offset)
private data class MapMarker(val point: Offset, val caption: String, val isDestination: Boolean)
private data class MapWall(val from: Offset, val to: Offset, val interior: Boolean)

private fun polygon(points: List<Offset>) = Path().apply {
    moveTo(points.first().x, points.first().y)
    points.drop(1).forEach { lineTo(it.x, it.y) }
    close()
}

private fun roomLabel(name: String, paint: Paint, width: Float): List<String> {
    val lines = mutableListOf<String>()
    var line = ""
    for (word in name.split(' ')) {
        if (paint.measureText(word) > width) return emptyList()
        val next = if (line.isEmpty()) word else "$line $word"
        if (paint.measureText(next) <= width) line = next
        else {
            lines.add(line)
            line = word
        }
    }
    if (line.isNotEmpty()) lines.add(line)
    return lines
}

private fun List<Offset>.containsPoint(point: Offset): Boolean {
    var inside = false
    var previous = last()
    for (next in this) {
        if ((next.y > point.y) != (previous.y > point.y)) {
            val x = next.x + (point.y - next.y) * (previous.x - next.x) / (previous.y - next.y)
            if (point.x < x) inside = !inside
        }
        previous = next
    }
    return inside
}

private fun roomColor(kind: String) = when (kind) {
    "corridor" -> Color(0xFF65766C)
    "entrance", "lobby" -> Color(0xFF4E6659)
    "vertical-circulation" -> Color(0xFF3F625D)
    "service" -> Color(0xFF495B58)
    "restricted" -> Color(0xFF303C3B)
    else -> Color(0xFF3D4F4A)
}

private fun groundColor(kind: String) = when (kind) {
    "lawn" -> Color(0xFF374A37)
    "planting" -> Color(0xFF344B36)
    "water" -> Color(0xFF255358)
    "paving" -> Color(0xFF52615A)
    "parking" -> Color(0xFF3C4845)
    "road" -> Color(0xFF333D3A)
    else -> Color(0xFF394A40)
}

private fun roomWalls(area: MapArea, portals: List<VenuePortal>): List<MapWall> {
    val clockwise = area.points.indices.sumOf { index ->
        val a = area.points[index]
        val b = area.points[(index + 1) % area.points.size]
        (a.x * b.y - b.x * a.y).toDouble()
    } > 0.0
    return buildList {
        area.points.forEachIndexed { index, from ->
            val to = area.points[(index + 1) % area.points.size]
            val edge = to - from
            val length = edge.getDistance()
            if (length == 0f) return@forEachIndexed
            val openings = portals.mapNotNull { portal ->
                val delta = Offset(portal.x, portal.y) - from
                val t = (delta.x * edge.x + delta.y * edge.y) / (length * length)
                val across = (delta - edge * t).getDistance()
                if (t !in 0f..1f || across > .12f) return@mapNotNull null
                val halfWidth = portal.width / (2 * length)
                (t - halfWidth).coerceAtLeast(0f)..(t + halfWidth).coerceAtMost(1f)
            }.sortedBy { it.start }
            val screenX = edge.x - edge.y * .28f
            val interior = if (clockwise) screenX > 0f else screenX < 0f
            var previous = 0f
            for (opening in openings) {
                if (opening.start > previous) add(MapWall(from + edge * previous, from + edge * opening.start, interior))
                previous = maxOf(previous, opening.endInclusive)
            }
            if (previous < 1f) add(MapWall(from + edge * previous, to, interior))
        }
    }
}

@Composable
fun VenueMap(
    catalog: VenueCatalog,
    route: JSONObject?,
    floorId: String,
    modifier: Modifier = Modifier,
    mode: MapViewMode = MapViewMode.DEPTH,
    gesturesEnabled: Boolean = true,
    stepNodeId: String? = null
) {
    val floors = remember(catalog) { catalog.raw.getJSONArray("floors").objects() }
    val floor = remember(floors, floorId) { floors.first { it.getString("id") == floorId } }
    val floorNames = remember(floors) { floors.associate { it.getString("id") to it.getString("name").substringBefore(" · ") } }
    val outline = remember(floor) { floor.getJSONArray("outline").points() }
    val spaces = remember(catalog, floorId) {
        catalog.raw.getJSONArray("spaces").objects().filter { it.getString("floorId") == floorId }.map {
            MapArea(it.getString("name"), it.getString("type"), it.getJSONArray("polygon").points(), it.getString("id"))
        }
    }
    val site = remember(catalog, floorId) { catalog.raw.optJSONObject("site")?.takeIf { it.getString("floorId") == floorId } }
    val grounds = remember(site) {
        site?.getJSONArray("grounds")?.objects().orEmpty().map {
            MapArea("", it.getString("kind"), it.getJSONArray("polygon").points())
        }
    }
    val buildings = remember(site) {
        site?.getJSONArray("buildings")?.objects().orEmpty().map {
            SiteBuilding(it.getString("name"), it.getInt("storeys"), it.getJSONArray("footprint").points())
        }
    }
    val features = remember(site) {
        site?.getJSONArray("features")?.objects().orEmpty().map {
            SiteFeature(it.getString("kind"), it.getJSONArray("position").mapPoint())
        }
    }
    val indoorRooms = remember(spaces, site, buildings) {
        spaces.filter { area ->
            site == null || buildings.any { building ->
                building.footprint.containsPoint(area.points.fold(Offset.Zero, Offset::plus) / area.points.size.toFloat())
            }
        }.toSet()
    }
    val walls = remember(indoorRooms, catalog.portals, floorId) {
        val portals = catalog.portals.filter { it.floorId == floorId }
        indoorRooms.filter { it.kind != "corridor" }.flatMap { room ->
            roomWalls(room, portals.filter { room.id in it.rooms })
        }
    }
    val nodes = remember(route) {
        route?.optJSONArray("path")?.objects().orEmpty().map { node ->
            MapNode(node.optString("id"), node.getString("floor"), Offset(node.getDouble("x").toFloat(), node.getDouble("y").toFloat()))
        }
    }
    val segments = remember(nodes, floorId) {
        nodes.zipWithNext().filter { (from, to) -> from.floor == floorId && to.floor == floorId }
    }
    val markers = remember(nodes, floorId, floorNames) {
        nodes.mapIndexedNotNull { index, node ->
            if (node.floor != floorId) return@mapIndexedNotNull null
            val next = nodes.getOrNull(index + 1)
            val caption = when {
                index == nodes.lastIndex -> "Destination"
                index == 0 -> "Start"
                next != null && next.floor != floorId -> "To ${floorNames[next.floor] ?: "next floor"}"
                else -> return@mapIndexedNotNull null
            }
            MapMarker(node.point, caption, index == nodes.lastIndex)
        }
    }
    val stepPoint = remember(nodes, floorId, stepNodeId) {
        stepNodeId?.let { id -> nodes.firstOrNull { it.id == id && it.floor == floorId }?.point }
    }
    val lineWidth = remember(route) {
        val clearance = route?.optJSONObject("displayClearance")
        clearance?.takeIf {
            it.optString("status") == "checked" && it.optBoolean("centerlineValid") && it.getJSONArray("issues").length() == 0
        }?.getDouble("widthMeters")?.toFloat()
    }

    val pitched = mode == MapViewMode.DEPTH
    fun plane(point: Offset): Offset =
        if (pitched) Offset(point.x - point.y * .28f, point.x * .20f + point.y * .82f) else point

    val bounds = remember(outline, spaces, grounds, buildings, pitched) {
        val points = (outline + spaces.flatMap { it.points } + grounds.flatMap { it.points } + buildings.flatMap { it.footprint }).map { plane(it) }
        Rect(points.minOf { it.x }, points.minOf { it.y }, points.maxOf { it.x }, points.maxOf { it.y })
    }
    val sectionDepth = floor.optDouble("clearHeight", 3.0).toFloat().coerceIn(2.2f, 6f)
    val labelPaint = remember {
        TextPaint(Paint.ANTI_ALIAS_FLAG).apply {
            textAlign = Paint.Align.CENTER
            typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
        }
    }
    val bubblePaint = remember { Paint(Paint.ANTI_ALIAS_FLAG) }
    var zoom by remember(catalog.hash, floorId, mode) { mutableFloatStateOf(1f) }
    var pan by remember(catalog.hash, floorId, mode) { mutableStateOf(Offset.Zero) }

    fun reset() {
        zoom = 1f
        pan = Offset.Zero
    }
    val gestures = if (gesturesEnabled) Modifier.pointerInput(catalog.hash, floorId, mode) {
        awaitEachGesture {
            awaitFirstDown()
            do {
                val event = awaitPointerEvent()
                if (event.changes.any { it.isConsumed }) break
                if (event.changes.count { it.pressed } >= 2 || zoom > 1f) {
                    val nextZoom = (zoom * event.calculateZoom()).coerceIn(1f, 6f)
                    val center = Offset(size.width / 2f, size.height / 2f)
                    val centroid = event.calculateCentroid(useCurrent = false)
                    if (centroid != Offset.Unspecified) {
                        val anchored = centroid - center + (pan + center - centroid) * (nextZoom / zoom)
                        val position = anchored + event.calculatePan()
                        pan = if (nextZoom == 1f) Offset.Zero else Offset(
                            position.x.coerceIn(-size.width * nextZoom / 2f, size.width * nextZoom / 2f),
                            position.y.coerceIn(-size.height * nextZoom / 2f, size.height * nextZoom / 2f)
                        )
                    }
                    zoom = nextZoom
                    event.changes.forEach { it.consume() }
                }
            } while (event.changes.any { it.pressed })
        }
    } else Modifier

    Box(modifier.fillMaxWidth().height(270.dp).clipToBounds()) {
        Canvas(Modifier.matchParentSize().then(gestures).semantics {
            contentDescription = if (gesturesEnabled) "Map of ${floor.getString("name")}. Pinch to zoom."
            else "Map of ${floor.getString("name")}."
            if (gesturesEnabled) customActions = buildList {
                if (zoom < 6f) add(CustomAccessibilityAction("Zoom in") { zoom = (zoom * 1.5f).coerceAtMost(6f); true })
                if (zoom > 1f) add(CustomAccessibilityAction("Zoom out") {
                    zoom = (zoom / 1.5f).coerceAtLeast(1f)
                    if (zoom == 1f) pan = Offset.Zero
                    true
                })
            }
        }) {
            val frameWidth = (size.width - 32.dp.toPx()).coerceAtLeast(1f)
            val frameHeight = (size.height - 34.dp.toPx()).coerceAtLeast(1f)
            val scale = min(frameWidth / bounds.width.coerceAtLeast(1f), frameHeight / bounds.height.coerceAtLeast(1f)) * zoom
            val offset = center + pan - bounds.center * scale
            fun project(p: Offset, height: Float = 0f) = offset + plane(p) * scale - Offset(0f, height * scale)
            fun projected(points: List<Offset>, height: Float = 0f) = points.map { project(it, height) }
            val floorPoints = projected(outline)
            val floorShape = polygon(floorPoints)

            drawRect(Brush.radialGradient(
                colors = listOf(Color(0xFF303E34), Color(0xFF1B241E), MapInk),
                center = Offset(size.width * .48f, size.height * .45f),
                radius = size.maxDimension * .68f
            ))
            if (pitched) {
                val shadow = Offset(3.dp.toPx(), 9.dp.toPx())
                drawPath(polygon(floorPoints.map { it + shadow }), MapShadow)
            }
            drawPath(floorShape, if (site != null) Color(0xFF314138) else Color(0xFF222F2A))
            grounds.forEach { ground ->
                val shape = polygon(projected(ground.points))
                drawPath(shape, groundColor(ground.kind))
                drawPath(shape, Color(0x3370836E), style = Stroke(.55.dp.toPx()))
            }

            if (site == null && pitched) {
                val drop = min(sectionDepth * scale * .35f, 14.dp.toPx())
                drawFrontFaces(floorPoints, drop, Color(0xFF303C39), Color(0xFF26312F))
                drawPath(floorShape, Color(0xFF293631))
            }

            buildings.forEach { building ->
                val points = projected(building.footprint)
                val shape = polygon(points)
                val cutaway = spaces.any { building.footprint.containsPoint(Rect(
                    it.points.minOf { p -> p.x }, it.points.minOf { p -> p.y },
                    it.points.maxOf { p -> p.x }, it.points.maxOf { p -> p.y }
                ).center) }
                val rise = if (cutaway) scale * .55f else scale * 2.9f * building.storeys
                val top = if (pitched) points.map { it - Offset(0f, rise) } else points
                if (pitched) {
                    drawPath(polygon(points.map { it + Offset(rise * .55f, rise * .8f) }), MapShadow)
                    drawFrontFaces(top, rise, Color(0xFF3D4948), Color(0xFF2E3B3A))
                }
                drawPath(if (pitched) polygon(top) else shape, Color(0xFF525F5A))
                drawPath(if (pitched) polygon(top) else shape, Color(0xFF7D8B80), style = Stroke(.65.dp.toPx()))
            }

            val screenRooms = spaces.map { it to projected(it.points) }
            screenRooms.forEach { (space, points) ->
                val shape = polygon(points)
                val bounds = shape.getBounds()
                val indoor = space in indoorRooms
                val color = if (indoor) roomColor(space.kind) else Color(0xFF506259)
                drawPath(shape, Brush.linearGradient(listOf(color, color.copy(alpha = .78f)), bounds.topLeft, bounds.bottomRight))
                if (!indoor) drawPath(shape, Color(0x4D778981), style = Stroke(.68.dp.toPx()))
            }
            walls.forEach { wall ->
                val from = project(wall.from)
                val to = project(wall.to)
                val height = if (wall.interior) .46f else .14f
                val rise = if (pitched) min(sectionDepth * scale * height, 17.dp.toPx()) else 0f
                drawRoomWall(from, to, rise, wall.interior)
            }

            features.forEach { feature ->
                val p = project(feature.position)
                val unit = scale.coerceAtMost(7.dp.toPx())
                if (feature.kind == "tree") {
                    val crown = if (pitched) p - Offset(0f, unit * 1.8f) else p
                    drawCircle(Color(0x660B1510), unit * 1.6f, p + Offset(unit, unit * .68f))
                    drawCircle(Color(0xFF314B33), unit * 1.5f, crown)
                    drawCircle(Color(0xFF547048), unit * 1.16f, crown - Offset(unit * .28f, unit * .26f))
                    drawCircle(Color(0xFF6B8153), unit * .64f, crown - Offset(unit * .46f, unit * .45f))
                } else {
                    drawCircle(if (feature.kind == "fountain") Color(0xFF94C5C8) else Color(0xFF8D9786), unit * .62f, p)
                }
            }

            val labels = mutableListOf<Rect>()
            val markerPoints = markers.map { project(it.point) }
            val segmentPoints = segments.map { (from, to) -> project(from.point) to project(to.point) }
            labelPaint.textSize = (if (gesturesEnabled) 10.sp else 9.sp).toPx()
            labelPaint.color = MapText.toArgb()
            val view = Rect(0f, 0f, size.width, size.height)
            screenRooms.filter { (area, _) ->
                area.kind !in setOf("corridor", "restricted") && (gesturesEnabled || area.kind in setOf("entrance", "lobby"))
            }.forEach { (area, points) ->
                val shapeBounds = polygon(points).getBounds()
                val lines = roomLabel(area.name, labelPaint, shapeBounds.width - 10.dp.toPx())
                val lineHeight = labelPaint.descent() - labelPaint.ascent()
                if (lines.isEmpty() || lines.size * lineHeight > shapeBounds.height - 4.dp.toPx()) return@forEach
                val labelBounds = Rect(
                    center = shapeBounds.center,
                    radius = lines.maxOf { labelPaint.measureText(it) } / 2f
                ).let { Rect(it.left, shapeBounds.center.y - lines.size * lineHeight / 2, it.right, shapeBounds.center.y + lines.size * lineHeight / 2) }
                if (view.overlaps(labelBounds) &&
                    listOf(labelBounds.topLeft, labelBounds.topRight, labelBounds.bottomLeft, labelBounds.bottomRight).all(points::containsPoint) &&
                    markers.none { labelBounds.inflate(22.dp.toPx()).contains(project(it.point)) } &&
                    segmentPoints.none { (from, to) -> lineIntersectsRect(from, to, labelBounds.inflate(3.dp.toPx())) } &&
                    labels.none { it.overlaps(labelBounds) }
                ) {
                    labels.add(labelBounds)
                    drawIntoCanvas { canvas ->
                        lines.forEachIndexed { index, line ->
                            canvas.nativeCanvas.drawText(line, labelBounds.center.x, labelBounds.top + index * lineHeight - labelPaint.ascent(), labelPaint)
                        }
                    }
                }
            }
            if (!gesturesEnabled && buildings.isNotEmpty()) {
                labelPaint.textSize = 10.sp.toPx()
                buildings.forEach { building ->
                    val p = polygon(projected(building.footprint)).getBounds().center
                    val width = labelPaint.measureText(building.name) / 2f + 6.dp.toPx()
                    val h = 10.dp.toPx()
                    val position = Rect(p.x - width, p.y - h, p.x + width, p.y + h)
                    drawMapCaption(position, building.name, MapInk.copy(alpha = .93f), MapText, labelPaint, bubblePaint)
                }
            }

            if (lineWidth != null) {
                // A pitched view compresses one axis. Use a lower bound of its
                // stretch so the stroke still fits in the checked display envelope.
                val stroke = (lineWidth * scale * if (pitched) .80f else 1f).coerceAtMost(9.dp.toPx())
                segmentPoints.forEach { (from, to) ->
                    drawLine(Color(0xFF293B24), from, to, strokeWidth = stroke, cap = StrokeCap.Round)
                    drawLine(RouteLime, from, to, strokeWidth = stroke * .76f, cap = StrokeCap.Round)
                }
            }
            stepPoint?.let { point ->
                val position = project(point)
                drawCircle(RouteLime.copy(alpha = .21f), 12.dp.toPx(), position)
                drawCircle(RouteLime, 8.dp.toPx(), position, style = Stroke(1.6.dp.toPx()))
            }
            markers.forEachIndexed { index, marker ->
                val point = markerPoints[index]
                if (!view.contains(point)) return@forEachIndexed
                val dot = if (marker.isDestination) Color(0xFFF6E6BE) else RouteLime
                drawCircle(MapInk, 6.3.dp.toPx(), point)
                drawCircle(dot, 4.4.dp.toPx(), point)

                labelPaint.textSize = 11.sp.toPx()
                val text = TextUtils.ellipsize(marker.caption, labelPaint, size.width * .42f, TextUtils.TruncateAt.END).toString()
                val half = labelPaint.measureText(text) / 2 + 8.dp.toPx()
                val h = labelPaint.descent() - labelPaint.ascent() + 6.dp.toPx()
                val gap = 9.dp.toPx()
                val horizontalPadding = 3.dp.toPx()
                val x = point.x.coerceIn(half + horizontalPadding, (size.width - half - horizontalPadding).coerceAtLeast(half + horizontalPadding))
                fun at(top: Float) = Rect(x - half, top, x + half, top + h)
                val candidates = listOf(at(point.y - h - gap), at(point.y + gap))
                val position = candidates.firstOrNull { bounds ->
                    bounds.top >= 0f && bounds.bottom <= size.height &&
                        labels.none(bounds::overlaps) && markerPoints.none { bounds.inflate(5.dp.toPx()).contains(it) }
                } ?: candidates.firstOrNull { it.top >= 0f && it.bottom <= size.height }
                if (position != null) {
                    labels.add(position)
                    drawMapCaption(position, text, dot, MapInk, labelPaint, bubblePaint)
                }
            }
        }

        if (gesturesEnabled && (zoom != 1f || pan != Offset.Zero)) {
            FilledTonalButton(onClick = ::reset, modifier = Modifier.align(Alignment.BottomEnd).padding(8.dp)) {
                Text("Reset map")
            }
        }
    }
}

private fun DrawScope.drawMapCaption(
    rect: Rect,
    text: String,
    background: Color,
    foreground: Color,
    labelPaint: TextPaint,
    bubblePaint: Paint
) {
    bubblePaint.color = background.toArgb()
    labelPaint.color = foreground.toArgb()
    val radius = min(rect.height / 2f, 12.dp.toPx())
    val baseline = rect.center.y - (labelPaint.ascent() + labelPaint.descent()) / 2
    drawIntoCanvas {
        it.nativeCanvas.drawRoundRect(RectF(rect.left, rect.top, rect.right, rect.bottom), radius, radius, bubblePaint)
        it.nativeCanvas.drawText(text, rect.center.x, baseline, labelPaint)
    }
}

private fun DrawScope.drawFrontFaces(points: List<Offset>, drop: Float, near: Color, side: Color) {
    // The lower half of the outline is visible from the same direction as the
    // pitch. Winding matters on venue packages with counterclockwise outlines.
    val clockwise = points.indices.sumOf { index ->
        val a = points[index]
        val b = points[(index + 1) % points.size]
        (a.x * b.y - b.x * a.y).toDouble()
    } > 0.0
    val step = Offset(0f, drop)
    points.forEachIndexed { index, from ->
        val to = points[(index + 1) % points.size]
        val directedX = if (clockwise) to.x - from.x else from.x - to.x
        if (directedX < 0f) {
            val color = if (kotlin.math.abs(to.y - from.y) > -directedX * .45f) side else near
            drawPath(polygon(listOf(from, to, to + step, from + step)), color)
        }
    }
}

private fun DrawScope.drawRoomWall(from: Offset, to: Offset, rise: Float, interior: Boolean) {
    val lift = Offset(0f, rise)
    val topFrom = from - lift
    val topTo = to - lift
    if (rise > 0f) {
        val face = if (interior) Color(0xFF78877F) else Color(0xFF445750)
        drawPath(polygon(listOf(from, to, topTo, topFrom)), face)
        drawLine(Color(0x770F1A16), from, to, strokeWidth = 1.2.dp.toPx())
    }
    val cap = if (interior && rise > 0f) Color(0xFFB3BAA9) else Color(0xFF8A9D90)
    drawLine(cap, topFrom, topTo, strokeWidth = .95.dp.toPx())
}

private fun lineIntersectsRect(a: Offset, b: Offset, rect: Rect): Boolean {
    // Test the part of the segment spanning the label's x range.
    val left = if (a.x <= b.x) a else b
    val right = if (a.x <= b.x) b else a
    if (left.x > rect.right || right.x < rect.left) return false
    if (left.x == right.x) return maxOf(left.y, right.y) >= rect.top && minOf(left.y, right.y) <= rect.bottom
    val slope = (right.y - left.y) / (right.x - left.x)
    val y1 = left.y + (maxOf(rect.left, left.x) - left.x) * slope
    val y2 = left.y + (minOf(rect.right, right.x) - left.x) * slope
    return maxOf(y1, y2) >= rect.top && minOf(y1, y2) <= rect.bottom
}
