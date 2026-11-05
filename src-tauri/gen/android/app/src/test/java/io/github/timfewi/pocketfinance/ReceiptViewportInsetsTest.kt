package io.github.timfewi.pocketfinance

import org.junit.Assert.assertEquals
import org.junit.Test

class ReceiptViewportInsetsTest {
  private val zero = ReceiptViewportInsets(0, 0, 0, 0)

  @Test
  fun keyboardAndNavigationBarDoNotAddTheirOverlappingHeights() {
    val bars = ReceiptViewportInsets(0, 24, 0, 34)
    val keyboard = ReceiptViewportInsets(0, 0, 0, 300)
    assertEquals(
      ReceiptViewportInsets(0, 24, 0, 300),
      receiptViewportMargins(zero, bars, zero, keyboard)
    )
  }

  @Test
  fun hidingKeyboardRestoresSystemInsetsWithoutAccumulatingMargins() {
    val original = ReceiptViewportInsets(2, 3, 4, 5)
    val bars = ReceiptViewportInsets(0, 24, 0, 34)
    assertEquals(
      ReceiptViewportInsets(2, 27, 4, 305),
      receiptViewportMargins(original, bars, zero, ReceiptViewportInsets(0, 0, 0, 300))
    )
    repeat(3) {
      assertEquals(
        ReceiptViewportInsets(2, 27, 4, 39),
        receiptViewportMargins(original, bars, zero, zero)
      )
    }
  }

  @Test
  fun landscapeCutoutAndSideNavigationStayOutsideTheViewport() {
    assertEquals(
      ReceiptViewportInsets(80, 24, 48, 0),
      receiptViewportMargins(
        zero,
        ReceiptViewportInsets(0, 24, 48, 0),
        ReceiptViewportInsets(80, 0, 0, 0),
        zero
      )
    )
  }
}
