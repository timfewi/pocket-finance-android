package dev.pocketfinance.receiptcapture

import org.junit.Assert.*
import org.junit.Test

class ReceiptFilesTest {
    @Test fun referencesAreOpaqueAndCanonical() {
        assertTrue(ReceiptFiles.validRef("01234567-89ab-4cde-8fab-0123456789ab"))
        for (ref in listOf("", "../receipt", "content://private/1", "01234567-89AB-4cde-8fab-0123456789ab")) {
            assertFalse(ReceiptFiles.validRef(ref))
        }
    }
    @Test fun previewSamplingStaysWithinBudget() {
        val sample = ReceiptFiles.sampleSize(8000, 6000, 1280)
        assertTrue(8000 / sample <= 1280)
        assertTrue(6000 / sample <= 1280)
        assertEquals(1, ReceiptFiles.sampleSize(600, 800, 1280))
    }
    @Test fun dimensionsRejectInvalidOrExcessiveImages() {
        assertFalse(ReceiptFiles.validDimensions(0, 10))
        assertFalse(ReceiptFiles.validDimensions(50000, 50000))
        assertTrue(ReceiptFiles.validDimensions(8000, 6000))
    }
}
