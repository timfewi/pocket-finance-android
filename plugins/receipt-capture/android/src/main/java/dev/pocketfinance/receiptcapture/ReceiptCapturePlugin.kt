package dev.pocketfinance.receiptcapture

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import androidx.activity.result.ActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import androidx.exifinterface.media.ExifInterface
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.io.File
import java.io.FileOutputStream
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicReference

@InvokeArg
class FileRefArgs { var fileRef: String = "" }

private class NativeFailure(val code: String) : Exception()
private class Operation(val invoke: Invoke, var fileRef: String? = null) {
    var outputUri: Uri? = null
    var timeout: Runnable? = null
    var removeOnFailure: Boolean = false
    @Volatile var cancelled: Boolean = false
}

@TauriPlugin
class ReceiptCapturePlugin(private val activity: Activity) : Plugin(activity) {
    private val worker = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private val active = AtomicReference<Operation?>(null)
    private var pendingCamera: Operation? = null
    private var pendingPicker: Operation? = null
    private val picker = ActivityResultContracts.PickVisualMedia()

    private fun root(): File {
        val base = activity.filesDir.canonicalFile
        val directory = File(base, "receipts")
        if (!directory.exists() && !directory.mkdir()) throw NativeFailure("unavailable")
        if (!directory.isDirectory || directory.canonicalFile.parentFile != base || directory.canonicalPath != directory.absolutePath) throw NativeFailure("invalidFile")
        return directory.canonicalFile
    }

    private fun ownedFile(ref: String, suffix: String): File {
        if (!ReceiptFiles.validRef(ref)) throw NativeFailure("invalidFile")
        val base = root()
        val file = File(base, ref + suffix)
        if (file.canonicalFile.parentFile != base || file.canonicalPath != file.absolutePath) throw NativeFailure("invalidFile")
        return file
    }

    private fun begin(invoke: Invoke): Operation? {
        val operation = Operation(invoke)
        if (pendingCamera != null || pendingPicker != null || !active.compareAndSet(null, operation)) {
            reject(invoke, "busy")
            return null
        }
        return operation
    }

    private fun reject(invoke: Invoke, code: String) {
        // Never attach exceptions: Tauri logs exception objects supplied to reject.
        val message = when (code) {
            "cancelled" -> "Receipt operation cancelled."
            "busy" -> "A receipt operation is already in progress."
            "invalidFile" -> "Receipt image is unavailable or invalid."
            "imageTooLarge" -> "Receipt image exceeds supported limits."
            "ocrFailed" -> "Text recognition failed. Enter the receipt manually."
            "timeout" -> "Text recognition timed out. Enter the receipt manually."
            else -> "Receipt capture is unavailable. Enter the receipt manually."
        }
        invoke.reject(message, code)
    }

    private fun removeFiles(ref: String, strict: Boolean = false) {
        for (suffix in listOf(".original", ".preview.jpg", ".capture.jpg")) {
            try {
                val file = ownedFile(ref, suffix)
                if (file.exists() && !file.delete() && strict) throw NativeFailure("unavailable")
            } catch (_: Exception) {
                if (strict) throw NativeFailure("unavailable")
            }
        }
    }

    private fun revoke(operation: Operation) {
        operation.outputUri?.let {
            try { activity.revokeUriPermission(it, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
            catch (_: Exception) { /* Camera grant may already be gone. */ }
        }
    }

    private fun finish(operation: Operation, result: JSObject? = null, code: String? = null) {
        main.post {
            if (!active.compareAndSet(operation, null)) {
                if (operation.cancelled && operation.removeOnFailure) operation.fileRef?.let { removeFiles(it) }
                return@post
            }
            operation.timeout?.let { main.removeCallbacks(it) }
            revoke(operation)
            if (code != null && operation.removeOnFailure) operation.fileRef?.let { removeFiles(it) }
            if (code == null) operation.invoke.resolve(result) else reject(operation.invoke, code)
        }
    }

    private fun checkActive(operation: Operation) {
        if (active.get() !== operation) throw NativeFailure("cancelled")
    }

    @Command
    fun storageRoot(invoke: Invoke) {
        try { invoke.resolve(JSObject().apply { put("root", root().absolutePath) }) }
        catch (_: Exception) { reject(invoke, "unavailable") }
    }

    @Command
    fun capture(invoke: Invoke) {
        val operation = begin(invoke) ?: return
        try {
            val ref = UUID.randomUUID().toString()
            operation.fileRef = ref
            operation.removeOnFailure = true
            val target = ownedFile(ref, ".capture.jpg")
            if (!target.createNewFile()) throw NativeFailure("invalidFile")
            val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.receiptcapture.files", target)
            operation.outputUri = uri
            val intent = Intent(MediaStore.ACTION_IMAGE_CAPTURE).apply {
                putExtra(MediaStore.EXTRA_OUTPUT, uri)
                clipData = ClipData.newRawUri("Receipt", uri)
                addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            if (intent.resolveActivity(activity.packageManager) == null) throw NativeFailure("unavailable")
            pendingCamera = operation
            startActivityForResult(invoke, intent, "captureResult")
        } catch (error: NativeFailure) { pendingCamera = null; finish(operation, code = error.code) }
        catch (_: Exception) { pendingCamera = null; finish(operation, code = "unavailable") }
    }

    @ActivityCallback
    fun captureResult(invoke: Invoke, result: ActivityResult) {
        val operation = pendingCamera ?: return
        pendingCamera = null
        if (operation.invoke !== invoke || active.get() !== operation) {
            operation.fileRef?.let { removeFiles(it) }
            revoke(operation)
            return
        }
        if (result.resultCode != Activity.RESULT_OK) {
            finish(operation, code = if (result.resultCode == Activity.RESULT_CANCELED) "cancelled" else "unavailable")
            return
        }
        worker.execute {
            try {
                checkActive(operation)
                val ref = operation.fileRef ?: throw NativeFailure("invalidFile")
                val captured = ownedFile(ref, ".capture.jpg")
                if (!captured.renameTo(ownedFile(ref, ".original"))) throw NativeFailure("invalidFile")
                val metadata = prepareImage(operation, ref)
                finish(operation, metadata)
            } catch (error: NativeFailure) { finish(operation, code = error.code) }
            catch (_: Exception) { finish(operation, code = "invalidFile") }
            catch (_: OutOfMemoryError) { finish(operation, code = "imageTooLarge") }
        }
    }

    @Command
    fun importReceipt(invoke: Invoke) {
        val operation = begin(invoke) ?: return
        try {
            // AndroidX chooses the system Photo Picker, supported backport, or
            // ACTION_OPEN_DOCUMENT fallback; no broad storage permission is needed.
            val intent = picker.createIntent(activity, PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
            pendingPicker = operation
            startActivityForResult(invoke, intent, "importResult")
        } catch (_: Exception) { pendingPicker = null; finish(operation, code = "unavailable") }
    }

    @ActivityCallback
    fun importResult(invoke: Invoke, result: ActivityResult) {
        val operation = pendingPicker ?: return
        pendingPicker = null
        if (operation.invoke !== invoke || active.get() !== operation) return
        val uri = picker.parseResult(result.resultCode, result.data)
        if (uri == null) { finish(operation, code = "cancelled"); return }
        // A selected provider is untrusted. Read only its granted content URI and
        // copy bytes into owned storage before decoding or returning a reference.
        if (uri.scheme != "content") { finish(operation, code = "invalidFile"); return }
        worker.execute {
            try {
                val ref = UUID.randomUUID().toString()
                operation.fileRef = ref
                operation.removeOnFailure = true
                val target = ownedFile(ref, ".original")
                activity.contentResolver.openInputStream(uri)?.use { input ->
                    FileOutputStream(target).use { output ->
                        val buffer = ByteArray(32 * 1024)
                        var total = 0L
                        while (true) {
                            checkActive(operation)
                            val read = input.read(buffer)
                            if (read < 0) break
                            total += read
                            if (total > ReceiptFiles.MAX_BYTES) throw NativeFailure("imageTooLarge")
                            output.write(buffer, 0, read)
                        }
                    }
                } ?: throw NativeFailure("invalidFile")
                finish(operation, prepareImage(operation, ref))
            } catch (error: NativeFailure) { finish(operation, code = error.code) }
            catch (_: Exception) { finish(operation, code = "invalidFile") }
            catch (_: OutOfMemoryError) { finish(operation, code = "imageTooLarge") }
        }
    }

    private fun imageBounds(file: File): BitmapFactory.Options {
        if (!file.isFile || file.length() == 0L) throw NativeFailure("invalidFile")
        if (file.length() > ReceiptFiles.MAX_BYTES) throw NativeFailure("imageTooLarge")
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(file.absolutePath, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) throw NativeFailure("invalidFile")
        if (!ReceiptFiles.validDimensions(bounds.outWidth, bounds.outHeight)) throw NativeFailure("imageTooLarge")
        if (bounds.outMimeType?.startsWith("image/") != true) throw NativeFailure("invalidFile")
        return bounds
    }

    private fun decodedOriented(file: File, maxEdge: Int): Bitmap {
        val bounds = imageBounds(file)
        val options = BitmapFactory.Options().apply {
            inSampleSize = ReceiptFiles.sampleSize(bounds.outWidth, bounds.outHeight, maxEdge)
            inPreferredConfig = Bitmap.Config.ARGB_8888
        }
        val bitmap = BitmapFactory.decodeFile(file.absolutePath, options) ?: throw NativeFailure("invalidFile")
        val orientation = try { ExifInterface(file).getAttributeInt(ExifInterface.TAG_ORIENTATION, ExifInterface.ORIENTATION_NORMAL) }
            catch (_: Exception) { ExifInterface.ORIENTATION_NORMAL }
        val matrix = Matrix()
        when (orientation) {
            ExifInterface.ORIENTATION_FLIP_HORIZONTAL -> matrix.setScale(-1f, 1f)
            ExifInterface.ORIENTATION_ROTATE_180 -> matrix.setRotate(180f)
            ExifInterface.ORIENTATION_FLIP_VERTICAL -> matrix.setScale(1f, -1f)
            ExifInterface.ORIENTATION_TRANSPOSE -> { matrix.setRotate(90f); matrix.postScale(-1f, 1f) }
            ExifInterface.ORIENTATION_ROTATE_90 -> matrix.setRotate(90f)
            ExifInterface.ORIENTATION_TRANSVERSE -> { matrix.setRotate(-90f); matrix.postScale(-1f, 1f) }
            ExifInterface.ORIENTATION_ROTATE_270 -> matrix.setRotate(-90f)
        }
        if (matrix.isIdentity) return bitmap
        val oriented = Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, matrix, true)
        if (oriented !== bitmap) bitmap.recycle()
        return oriented
    }

    private fun prepareImage(operation: Operation, ref: String): JSObject {
        checkActive(operation)
        val file = ownedFile(ref, ".original")
        val bounds = imageBounds(file)
        val preview = decodedOriented(file, 1280)
        try {
            checkActive(operation)
            FileOutputStream(ownedFile(ref, ".preview.jpg")).use { stream ->
                if (!preview.compress(Bitmap.CompressFormat.JPEG, 85, stream)) throw NativeFailure("invalidFile")
            }
            return JSObject().apply {
                put("fileRef", ref)
                put("previewRef", ref)
                put("width", bounds.outWidth)
                put("height", bounds.outHeight)
                put("mimeType", bounds.outMimeType)
                put("previewWidth", preview.width)
                put("previewHeight", preview.height)
            }
        } finally { preview.recycle() }
    }

    @Command
    fun recognize(invoke: Invoke) {
        val operation = begin(invoke) ?: return
        try {
            val ref = invoke.parseArgs(FileRefArgs::class.java).fileRef
            if (!ReceiptFiles.validRef(ref)) throw NativeFailure("invalidFile")
            operation.fileRef = ref
            val timeout = Runnable { finish(operation, code = "timeout") }
            operation.timeout = timeout
            main.postDelayed(timeout, 30_000L)
            worker.execute {
                var bitmap: Bitmap? = null
                try {
                    checkActive(operation)
                    val decoded = decodedOriented(ownedFile(ref, ".original"), 4096)
                    bitmap = decoded
                    checkActive(operation)
                    val image = InputImage.fromBitmap(decoded, 0)
                    val recognizer = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)
                    try {
                        recognizer.process(image)
                            .addOnSuccessListener { result ->
                                if (result.text.length > 64_000) finish(operation, code = "ocrFailed")
                                else finish(operation, JSObject().apply { put("text", result.text) })
                            }
                            .addOnFailureListener { finish(operation, code = "ocrFailed") }
                            .addOnCompleteListener {
                                try { recognizer.close() } finally { decoded.recycle() }
                            }
                        bitmap = null // Task completion owns this bitmap now.
                    } catch (error: Exception) { recognizer.close(); throw error }
                } catch (error: NativeFailure) { bitmap?.recycle(); finish(operation, code = error.code) }
                catch (_: Exception) { bitmap?.recycle(); finish(operation, code = "ocrFailed") }
                catch (_: OutOfMemoryError) { bitmap?.recycle(); finish(operation, code = "imageTooLarge") }
            }
        } catch (error: NativeFailure) { finish(operation, code = error.code) }
        catch (_: Exception) { finish(operation, code = "invalidFile") }
    }

    @Command
    fun discard(invoke: Invoke) {
        try {
            val ref = invoke.parseArgs(FileRefArgs::class.java).fileRef
            if (!ReceiptFiles.validRef(ref)) throw NativeFailure("invalidFile")
            if (active.get()?.fileRef == ref || pendingCamera?.fileRef == ref) throw NativeFailure("busy")
            removeFiles(ref, strict = true)
            invoke.resolve()
        } catch (error: NativeFailure) { reject(invoke, error.code) }
        catch (_: Exception) { reject(invoke, "invalidFile") }
    }

    @Command
    fun cancel(invoke: Invoke) {
        val operation = active.getAndSet(null)
        operation?.let {
            it.cancelled = true
            it.timeout?.let(main::removeCallbacks)
            revoke(it)
            if (it.removeOnFailure) it.fileRef?.let { ref -> removeFiles(ref) }
            reject(it.invoke, "cancelled")
        }
        invoke.resolve()
    }

    override fun onDestroy(activity: AppCompatActivity) {
        val operation = active.getAndSet(null)
        operation?.let {
            it.cancelled = true
            it.timeout?.let(main::removeCallbacks)
            revoke(it)
            if (it.removeOnFailure) it.fileRef?.let { ref -> removeFiles(ref) }
            reject(it.invoke, "cancelled")
        }
        worker.shutdownNow()
        super.onDestroy(activity)
    }
}
