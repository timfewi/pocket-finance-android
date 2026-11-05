package dev.pocketfinance.receiptcapture

import androidx.core.content.FileProvider

/** Distinct manifest component, so host providers cannot merge over this scope. */
class ReceiptFileProvider : FileProvider()
