package io.github.timfewi.pocketfinance

/** Pixel insets at the native boundary; no display-density conversion is needed. */
internal data class ReceiptViewportInsets(
  val left: Int,
  val top: Int,
  val right: Int,
  val bottom: Int
)

/** Overlapping system/keyboard occlusions use the largest inset, not their sum. */
internal fun receiptViewportMargins(
  original: ReceiptViewportInsets,
  systemBars: ReceiptViewportInsets,
  cutout: ReceiptViewportInsets,
  keyboard: ReceiptViewportInsets
): ReceiptViewportInsets = ReceiptViewportInsets(
  original.left + maxOf(0, systemBars.left, cutout.left, keyboard.left),
  original.top + maxOf(0, systemBars.top, cutout.top, keyboard.top),
  original.right + maxOf(0, systemBars.right, cutout.right, keyboard.right),
  original.bottom + maxOf(0, systemBars.bottom, cutout.bottom, keyboard.bottom)
)
