package dev.pocketfinance.receiptcapture

/** Pure validation helpers, shared by storage and JVM tests. */
internal object ReceiptFiles {
    const val MAX_BYTES = 32L * 1024L * 1024L
    const val MAX_PIXELS = 100_000_000L
    fun validRef(value: String): Boolean = value.length == 36 && value.withIndex().all { (i, c) ->
        if (i == 8 || i == 13 || i == 18 || i == 23) c == '-'
        else c in '0'..'9' || c in 'a'..'f'
    }
    fun sampleSize(width: Int, height: Int, maxEdge: Int): Int {
        require(width > 0 && height > 0 && maxEdge > 0)
        var sample = 1
        while (width / sample > maxEdge || height / sample > maxEdge) sample *= 2
        return sample
    }
    fun validDimensions(width: Int, height: Int): Boolean =
        width in 1..50_000 && height in 1..50_000 && width.toLong() * height <= MAX_PIXELS
}
