package app.wayform

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

internal val Paper = Color(0xFF111614)
internal val Forest = Color(0xFF1C2822)
internal val Lime = Color(0xFFC8FA73)
internal val Muted = Color(0xFFB2BEB4)
private val Ink = Color(0xFF17200C)
private val Text = Color(0xFFF2F6EE)

private val WayformColors = darkColorScheme(
    primary = Lime,
    onPrimary = Ink,
    primaryContainer = Color(0xFF354924),
    onPrimaryContainer = Color(0xFFD9F8BD),
    secondary = Color(0xFFB8CDAE),
    onSecondary = Ink,
    secondaryContainer = Color(0xFF34442D),
    onSecondaryContainer = Text,
    background = Paper,
    onBackground = Text,
    surface = Color(0xFF1A211D),
    onSurface = Text,
    surfaceVariant = Color(0xFF28352D),
    onSurfaceVariant = Muted,
    surfaceContainer = Color(0xFF202823),
    surfaceContainerHigh = Forest,
    surfaceContainerHighest = Color(0xFF28332B),
    outline = Color(0xFF84958A),
    outlineVariant = Color(0xFF39463E),
    error = Color(0xFFFFB6A8),
    onError = Color(0xFF36140F),
    errorContainer = Color(0xFF492A25),
    onErrorContainer = Color(0xFFFFDCD5),
)

private val WayformShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(18.dp),
    large = RoundedCornerShape(24.dp),
    extraLarge = RoundedCornerShape(30.dp),
)

@Composable
internal fun WayformTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = WayformColors, shapes = WayformShapes, content = content)
}
