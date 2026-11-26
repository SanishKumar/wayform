package app.wayform

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

internal val Paper = Color(0xFF121317)
private val RaisedSurface = Color(0xFF25262D)
internal val Lime = Color(0xFFC8FA73)
internal val Muted = Color(0xFFB8BAC3)
private val Ink = Color(0xFF171D10)
private val OffWhite = Color(0xFFF4F4F6)

private val WayformColors = darkColorScheme(
    primary = Lime,
    onPrimary = Ink,
    primaryContainer = Color(0xFF333F27),
    onPrimaryContainer = Lime,
    secondary = Color(0xFFD6D7DD),
    onSecondary = Color(0xFF23252C),
    secondaryContainer = Color(0xFF34363F),
    onSecondaryContainer = OffWhite,
    background = Paper,
    onBackground = OffWhite,
    surface = Color(0xFF1C1D23),
    onSurface = OffWhite,
    surfaceVariant = Color(0xFF2A2C34),
    onSurfaceVariant = Muted,
    surfaceContainer = Color(0xFF22232A),
    surfaceContainerHigh = RaisedSurface,
    surfaceContainerHighest = Color(0xFF30323B),
    outline = Color(0xFF858895),
    outlineVariant = Color(0xFF3B3D48),
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
