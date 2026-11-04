package app.wayform

import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayInputStream

class InputLimitsTest {
    @Test fun boundedReadsAcceptTheBoundaryAndRejectOneExtraByte() {
        assertArrayEquals(byteArrayOf(1, 2, 3), ByteArrayInputStream(byteArrayOf(1, 2, 3)).readBounded(3))
        assertThrows(IllegalArgumentException::class.java) { ByteArrayInputStream(byteArrayOf(1, 2, 3, 4)).readBounded(3) }
    }
    @Test fun nonFiniteOrInvalidRequirementsCannotBeAccepted() {
        for (value in listOf("NaN", "Infinity", "-1", "garbage", "501")) assertNotNull(MobilityNeeds(widthCm = value).validationError())
        assertNotNull(MobilityNeeds(slopePercent = "101").validationError())
        assertNull(MobilityNeeds(widthCm = "90", slopePercent = "0").validationError())
        assertTrue(MobilityNeeds(widthCm = "90").hasRequirements)
        assertTrue(MobilityNeeds(noSteps = true).hasRequirements)
        assertFalse(MobilityNeeds().hasRequirements)
    }
}
