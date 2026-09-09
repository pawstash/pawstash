package app.pawstash.client

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.MediaMetadataRetriever
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import java.io.File

class DownloadForegroundService : Service() {

    companion object {
        private const val TAG = "PawstashService"
        const val CHANNEL_DOWNLOADS_ID = "pawstash_downloads"
        const val CHANNEL_COMPLETED_ID = "pawstash_completed"
        const val FOREGROUND_NOTIFICATION_ID = 1001
        private const val BASE_COMPLETED_NOTIFICATION_ID = 2000

        const val ACTION_PAUSE = "app.pawstash.client.ACTION_PAUSE"
        const val ACTION_RESUME = "app.pawstash.client.ACTION_RESUME"
        const val ACTION_CANCEL = "app.pawstash.client.ACTION_CANCEL"
        const val ACTION_PAUSED_STATE = "app.pawstash.client.ACTION_PAUSED_STATE"

        init {
            try {
                System.loadLibrary("pawstash_lib")
            } catch (_: Throwable) {}
        }

        @JvmStatic
        external fun onNotificationAction(action: String)

        private var completedNotificationCount = 0
        private var wakeLock: PowerManager.WakeLock? = null
        private var lastUpdateTime = 0L

        @Volatile
        var isServiceRunning = false
            private set

        @Volatile
        var isPaused = false
            private set

        private var lastActiveCount = 1
        private var lastTotalCount = 1
        private var lastDownloadedBytes = 0L
        private var lastTotalBytes = 0L
        private var lastSpeedBytesPerSec = 0L
        private var lastCurrentFilename = "File"

        @Volatile
        private var activeServiceInstance: DownloadForegroundService? = null

        fun updateProgress(
            context: Context,
            activeCount: Int,
            totalCount: Int,
            downloadedBytes: Long,
            totalBytes: Long,
            speedBytesPerSec: Long,
            currentFilename: String
        ) {
            lastActiveCount = activeCount
            lastTotalCount = totalCount
            lastDownloadedBytes = downloadedBytes
            lastTotalBytes = totalBytes
            lastSpeedBytesPerSec = speedBytesPerSec
            lastCurrentFilename = currentFilename
            isPaused = false

            val now = System.currentTimeMillis()
            if (isServiceRunning && now - lastUpdateTime < 300 && totalBytes > 0 && downloadedBytes < totalBytes) {
                return
            }
            lastUpdateTime = now

            val service = activeServiceInstance
            if (service != null && isServiceRunning) {
                service.renderProgressNotification(
                    activeCount,
                    totalCount,
                    downloadedBytes,
                    totalBytes,
                    speedBytesPerSec,
                    currentFilename
                )
            } else {
                Log.d(TAG, "Starting foreground service via startForegroundService")
                val intent = Intent(context, DownloadForegroundService::class.java).apply {
                    action = "ACTION_UPDATE_PROGRESS"
                    putExtra("activeCount", activeCount)
                    putExtra("totalCount", totalCount)
                    putExtra("downloadedBytes", downloadedBytes)
                    putExtra("totalBytes", totalBytes)
                    putExtra("speedBytesPerSec", speedBytesPerSec)
                    putExtra("currentFilename", currentFilename)
                }
                try {
                    ContextCompat.startForegroundService(context, intent)
                } catch (e: Throwable) {
                    Log.e(TAG, "Failed to start foreground service", e)
                }
            }
        }

        fun updatePaused(context: Context, pausedCount: Int) {
            isPaused = true
            val service = activeServiceInstance
            if (service != null && isServiceRunning) {
                service.renderPausedNotification(pausedCount)
            } else {
                val intent = Intent(context, DownloadForegroundService::class.java).apply {
                    action = ACTION_PAUSED_STATE
                    putExtra("pausedCount", pausedCount)
                }
                try {
                    ContextCompat.startForegroundService(context, intent)
                } catch (e: Throwable) {
                    Log.e(TAG, "Failed to update paused state", e)
                }
            }
        }

        fun notifyCompleted(
            context: Context,
            service: String,
            creatorId: String,
            creatorName: String,
            postId: String,
            filename: String,
            title: String,
            finalPath: String,
            previewPath: String,
            sound: Boolean,
            showPreview: Boolean
        ) {
            try {
                ensureNotificationChannels(context)
                val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

                val notifId = BASE_COMPLETED_NOTIFICATION_ID + (++completedNotificationCount % 100)
                val pendingIntentFlags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                } else {
                    PendingIntent.FLAG_UPDATE_CURRENT
                }

                val postIntent = Intent(context, MainActivity::class.java).apply {
                    addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
                    putExtra("deep_link_action", "open_post")
                    putExtra("deep_link_service", service)
                    putExtra("deep_link_creator_id", creatorId)
                    putExtra("deep_link_post_id", postId)
                }
                val postPendingIntent = PendingIntent.getActivity(context, notifId, postIntent, pendingIntentFlags)

                val file = if (finalPath.isNotBlank()) File(finalPath) else null
                val filePendingIntent = if (file != null && file.exists()) {
                    try {
                        val uri = androidx.core.content.FileProvider.getUriForFile(
                            context,
                            "${context.packageName}.fileprovider",
                            file
                        )
                        val ext = file.extension.lowercase()
                        val mimeType = when (ext) {
                            "mp4", "m4v", "mkv", "webm", "mov", "avi", "3gp", "ts" -> "video/*"
                            "mp3", "m4a", "aac", "flac", "ogg", "opus", "wav" -> "audio/*"
                            "png", "jpg", "jpeg", "webp", "gif", "avif", "bmp" -> "image/*"
                            "pdf" -> "application/pdf"
                            "zip", "rar", "7z", "tar", "gz" -> "application/zip"
                            else -> "*/*"
                        }
                        val viewIntent = Intent(Intent.ACTION_VIEW).apply {
                            setDataAndType(uri, mimeType)
                            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
                        }
                        val chooser = Intent.createChooser(viewIntent, "Open with").apply {
                            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                        }
                        PendingIntent.getActivity(context, notifId + 1000, chooser, pendingIntentFlags)
                    } catch (_: Throwable) {
                        null
                    }
                } else null

                val folderPendingIntent = if (file != null && file.exists() && file.parent != null) {
                    val folderIntent = Intent(context, MainActivity::class.java).apply {
                        addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
                        putExtra("deep_link_action", "open_folder")
                        putExtra("folder_path", file.parent)
                    }
                    PendingIntent.getActivity(context, notifId + 2000, folderIntent, pendingIntentFlags)
                } else null

                val displayTitle = if (title.isNotBlank()) title else filename
                val contentTitle = if (creatorName.isNotBlank() && !displayTitle.contains(creatorName, ignoreCase = true)) {
                    "$creatorName • $displayTitle"
                } else {
                    displayTitle
                }
                val contentText = filename
                val serviceTag = if (service.isNotBlank()) service.replaceFirstChar { it.uppercase() } else "Pawstash"

                var previewBitmap: Bitmap? = null
                if (showPreview) {
                    try {
                        val isImage = file != null && file.exists() && file.length() > 0 &&
                            file.extension.lowercase() in listOf("jpg", "jpeg", "png", "webp", "avif", "bmp", "gif")
                        if (isImage && file != null) {
                            previewBitmap = decodeSampledBitmap(file.absolutePath, 1024, 1024)
                        } else {
                            val prevFile = if (previewPath.isNotBlank()) File(previewPath) else null
                            if (prevFile != null && prevFile.exists() && prevFile.length() > 0) {
                                previewBitmap = decodeSampledBitmap(prevFile.absolutePath, 1024, 1024)
                            } else if (file != null && file.exists() && file.extension.lowercase() in listOf("mp4", "mkv", "webm", "mov", "m4v")) {
                                previewBitmap = createVideoThumbnail(file.absolutePath)
                            }
                        }
                    } catch (e: Throwable) {
                        Log.w(TAG, "Failed to decode preview bitmap for notification", e)
                    }
                }

                val notificationBuilder = NotificationCompat.Builder(context, CHANNEL_COMPLETED_ID)
                    .setSmallIcon(R.drawable.ic_notification)
                    .setContentTitle(contentTitle)
                    .setContentText(contentText)
                    .setSubText(serviceTag)
                    .setAutoCancel(true)
                    .setContentIntent(postPendingIntent)

                if (sound) {
                    notificationBuilder.setDefaults(NotificationCompat.DEFAULT_SOUND or NotificationCompat.DEFAULT_VIBRATE)
                        .setPriority(NotificationCompat.PRIORITY_DEFAULT)
                } else {
                    notificationBuilder.setSilent(true)
                        .setSound(null)
                        .setVibrate(null)
                        .setPriority(NotificationCompat.PRIORITY_LOW)
                }

                if (previewBitmap != null) {
                    notificationBuilder.setLargeIcon(previewBitmap)
                    notificationBuilder.setStyle(
                        NotificationCompat.BigPictureStyle()
                            .bigPicture(previewBitmap)
                            .setSummaryText(contentText)
                    )
                }

                notificationBuilder.addAction(0, "Open Post", postPendingIntent)
                if (filePendingIntent != null) {
                    notificationBuilder.addAction(0, "Open File", filePendingIntent)
                }
                if (folderPendingIntent != null) {
                    notificationBuilder.addAction(0, "Show in Folder", folderPendingIntent)
                }

                manager.notify(notifId, notificationBuilder.build())
                Log.d(TAG, "Completed notification posted: $contentTitle")
            } catch (e: Throwable) {
                Log.e(TAG, "Failed to post completed notification", e)
            }
        }

        fun stopService(context: Context) {
            val intent = Intent(context, DownloadForegroundService::class.java).apply {
                action = "ACTION_STOP_SERVICE"
            }
            try {
                context.startService(intent)
            } catch (e: Throwable) {
                Log.e(TAG, "Failed to stop service", e)
            }
        }

        fun ensureNotificationChannels(context: Context) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

                if (manager.getNotificationChannel(CHANNEL_DOWNLOADS_ID) == null) {
                    val channel = NotificationChannel(
                        CHANNEL_DOWNLOADS_ID,
                        "Downloads Progress",
                        NotificationManager.IMPORTANCE_LOW
                    ).apply {
                        description = "Shows live download progress and speed"
                        setShowBadge(false)
                        setSound(null, null)
                        enableVibration(false)
                    }
                    manager.createNotificationChannel(channel)
                }

                if (manager.getNotificationChannel(CHANNEL_COMPLETED_ID) == null) {
                    val channel = NotificationChannel(
                        CHANNEL_COMPLETED_ID,
                        "Completed Downloads",
                        NotificationManager.IMPORTANCE_LOW
                    ).apply {
                        description = "Notifications when downloads finish"
                        setShowBadge(false)
                        setSound(null, null)
                        enableVibration(false)
                    }
                    manager.createNotificationChannel(channel)
                }
            }
        }

        private fun formatBytes(bytes: Long): String {
            if (bytes <= 0) return "0 B"
            val kb = bytes / 1024.0
            val mb = kb / 1024.0
            val gb = mb / 1024.0
            return when {
                gb >= 1.0 -> String.format("%.2f GB", gb)
                mb >= 1.0 -> String.format("%.1f MB", mb)
                kb >= 1.0 -> String.format("%.1f KB", kb)
                else -> "$bytes B"
            }
        }

        private fun formatSpeed(bytesPerSec: Long): String {
            if (bytesPerSec <= 0) return ""
            return "${formatBytes(bytesPerSec)}/s"
        }

        private fun decodeSampledBitmap(path: String, reqWidth: Int, reqHeight: Int): Bitmap? {
            return try {
                val options = BitmapFactory.Options().apply {
                    inJustDecodeBounds = true
                }
                BitmapFactory.decodeFile(path, options)
                if (options.outWidth <= 0 || options.outHeight <= 0) return null

                var inSampleSize = 1
                val height = options.outHeight
                val width = options.outWidth
                if (height > reqHeight || width > reqWidth) {
                    val halfHeight = height / 2
                    val halfWidth = width / 2
                    while ((halfHeight / inSampleSize) >= reqHeight && (halfWidth / inSampleSize) >= reqWidth) {
                        inSampleSize *= 2
                    }
                }
                val decodeOptions = BitmapFactory.Options().apply {
                    this.inSampleSize = inSampleSize
                    inPreferredConfig = Bitmap.Config.ARGB_8888
                }
                BitmapFactory.decodeFile(path, decodeOptions)
            } catch (_: Throwable) {
                null
            }
        }

        private fun createVideoThumbnail(videoPath: String): Bitmap? {
            return try {
                val retriever = MediaMetadataRetriever()
                retriever.setDataSource(videoPath)
                val bitmap = retriever.getFrameAtTime(1000000, MediaMetadataRetriever.OPTION_CLOSEST_SYNC)
                    ?: retriever.frameAtTime
                retriever.release()
                bitmap
            } catch (_: Throwable) {
                null
            }
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        ensureNotificationChannels(this)
        acquireWakeLock()
        activeServiceInstance = this
        isServiceRunning = true

        val initialNotification = NotificationCompat.Builder(this, CHANNEL_DOWNLOADS_ID)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle("Pawstash Downloads")
            .setContentText("Starting download...")
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setSound(null)
            .setVibrate(null)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .build()

        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    FOREGROUND_NOTIFICATION_ID,
                    initialNotification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
                )
            } else {
                startForeground(FOREGROUND_NOTIFICATION_ID, initialNotification)
            }
            Log.d(TAG, "DownloadForegroundService started in foreground")
        } catch (e: Throwable) {
            Log.e(TAG, "Error in startForeground during onCreate", e)
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent == null) return START_NOT_STICKY

        when (intent.action) {
            "ACTION_STOP_SERVICE" -> {
                shutdownService()
                return START_NOT_STICKY
            }
            ACTION_PAUSE -> {
                isPaused = true
                releaseWakeLock()
                renderPausedNotification(lastTotalCount.coerceAtLeast(1))
                try {
                    onNotificationAction("pause")
                } catch (e: Throwable) {
                    Log.e(TAG, "Error invoking onNotificationAction(pause)", e)
                }
                return START_STICKY
            }
            ACTION_RESUME -> {
                isPaused = false
                acquireWakeLock()
                val isRu = java.util.Locale.getDefault().language.equals("ru", ignoreCase = true)
                val resumingText = if (isRu) "Возобновление..." else "Resuming..."
                renderProgressNotification(
                    lastActiveCount,
                    lastTotalCount,
                    lastDownloadedBytes,
                    lastTotalBytes,
                    0L,
                    resumingText
                )
                try {
                    onNotificationAction("resume")
                } catch (e: Throwable) {
                    Log.e(TAG, "Error invoking onNotificationAction(resume)", e)
                }
                return START_STICKY
            }
            ACTION_CANCEL -> {
                isPaused = false
                try {
                    onNotificationAction("cancel")
                } catch (e: Throwable) {
                    Log.e(TAG, "Error invoking onNotificationAction(cancel)", e)
                }
                shutdownService()
                return START_NOT_STICKY
            }
            ACTION_PAUSED_STATE -> {
                val pausedCount = intent.getIntExtra("pausedCount", 1)
                isPaused = true
                releaseWakeLock()
                renderPausedNotification(pausedCount)
                return START_STICKY
            }
            "ACTION_UPDATE_PROGRESS" -> {
                isPaused = false
                acquireWakeLock()
                val activeCount = intent.getIntExtra("activeCount", 1)
                val totalCount = intent.getIntExtra("totalCount", 1)
                val downloadedBytes = intent.getLongExtra("downloadedBytes", 0L)
                val totalBytes = intent.getLongExtra("totalBytes", 0L)
                val speedBytesPerSec = intent.getLongExtra("speedBytesPerSec", 0L)
                val currentFilename = intent.getStringExtra("currentFilename") ?: "File"

                renderProgressNotification(
                    activeCount,
                    totalCount,
                    downloadedBytes,
                    totalBytes,
                    speedBytesPerSec,
                    currentFilename
                )
            }
        }

        return START_STICKY
    }

    fun renderProgressNotification(
        activeCount: Int,
        totalCount: Int,
        downloadedBytes: Long,
        totalBytes: Long,
        speedBytesPerSec: Long,
        currentFilename: String
    ) {
        val title = if (totalCount > 1) {
            "Downloading $activeCount of $totalCount files"
        } else {
            "Downloading: $currentFilename"
        }

        val speed = formatSpeed(speedBytesPerSec)
        val sizeText = if (totalBytes > 0) {
            "${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)}"
        } else {
            formatBytes(downloadedBytes)
        }

        val contentText = if (speed.isNotEmpty()) {
            "$sizeText • $speed"
        } else {
            sizeText
        }

        val progressPercent = if (totalBytes > 0) {
            ((downloadedBytes.toDouble() / totalBytes.toDouble()) * 100).toInt().coerceIn(0, 100)
        } else {
            0
        }
        val isIndeterminate = totalBytes <= 0

        val openIntent = Intent(this, MainActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
            putExtra("deep_link_action", "open_downloads")
        }

        val pendingFlags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        } else {
            PendingIntent.FLAG_UPDATE_CURRENT
        }
        val pendingIntent = PendingIntent.getActivity(this, FOREGROUND_NOTIFICATION_ID, openIntent, pendingFlags)

        val isRu = java.util.Locale.getDefault().language.equals("ru", ignoreCase = true)
        val pauseIntent = Intent(this, DownloadForegroundService::class.java).apply {
            action = ACTION_PAUSE
        }
        val pausePendingIntent = PendingIntent.getService(this, 101, pauseIntent, pendingFlags)

        val cancelIntent = Intent(this, DownloadForegroundService::class.java).apply {
            action = ACTION_CANCEL
        }
        val cancelPendingIntent = PendingIntent.getService(this, 103, cancelIntent, pendingFlags)

        val pauseLabel = if (isRu) "Пауза" else "Pause"
        val cancelLabel = if (isRu) "Отмена" else "Cancel"

        val notification = NotificationCompat.Builder(this, CHANNEL_DOWNLOADS_ID)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(title)
            .setContentText(contentText)
            .setProgress(100, progressPercent, isIndeterminate)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setSound(null)
            .setVibrate(null)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .setContentIntent(pendingIntent)
            .addAction(0, pauseLabel, pausePendingIntent)
            .addAction(0, cancelLabel, cancelPendingIntent)
            .build()

        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    FOREGROUND_NOTIFICATION_ID,
                    notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
                )
            } else {
                startForeground(FOREGROUND_NOTIFICATION_ID, notification)
            }
        } catch (e: Throwable) {
            Log.e(TAG, "Error updating foreground notification", e)
        }
    }

    fun renderPausedNotification(pausedCount: Int) {
        val isRu = java.util.Locale.getDefault().language.equals("ru", ignoreCase = true)
        val title = if (isRu) "Загрузки приостановлены" else "Downloads paused"
        val countText = if (pausedCount > 1) {
            if (isRu) "Приостановлено файлов: $pausedCount" else "$pausedCount downloads paused"
        } else {
            if (isRu) "Загрузка приостановлена" else "Download paused"
        }

        val openIntent = Intent(this, MainActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
            putExtra("deep_link_action", "open_downloads")
        }
        val pendingFlags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        } else {
            PendingIntent.FLAG_UPDATE_CURRENT
        }
        val pendingIntent = PendingIntent.getActivity(this, FOREGROUND_NOTIFICATION_ID, openIntent, pendingFlags)

        val resumeIntent = Intent(this, DownloadForegroundService::class.java).apply {
            action = ACTION_RESUME
        }
        val resumePendingIntent = PendingIntent.getService(this, 102, resumeIntent, pendingFlags)

        val cancelIntent = Intent(this, DownloadForegroundService::class.java).apply {
            action = ACTION_CANCEL
        }
        val cancelPendingIntent = PendingIntent.getService(this, 103, cancelIntent, pendingFlags)

        val resumeLabel = if (isRu) "Продолжить" else "Resume"
        val cancelLabel = if (isRu) "Отмена" else "Cancel"

        val progressPercent = if (lastTotalBytes > 0) {
            ((lastDownloadedBytes.toDouble() / lastTotalBytes.toDouble()) * 100).toInt().coerceIn(0, 100)
        } else {
            0
        }

        val notification = NotificationCompat.Builder(this, CHANNEL_DOWNLOADS_ID)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(title)
            .setContentText(countText)
            .setProgress(100, progressPercent, false)
            .setOngoing(false)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setSound(null)
            .setVibrate(null)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .setContentIntent(pendingIntent)
            .addAction(0, resumeLabel, resumePendingIntent)
            .addAction(0, cancelLabel, cancelPendingIntent)
            .build()

        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    FOREGROUND_NOTIFICATION_ID,
                    notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
                )
            } else {
                startForeground(FOREGROUND_NOTIFICATION_ID, notification)
            }
        } catch (e: Throwable) {
            Log.e(TAG, "Error updating paused foreground notification", e)
        }
    }

    private fun acquireWakeLock() {
        try {
            if (wakeLock == null) {
                val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
                wakeLock = powerManager.newWakeLock(
                    PowerManager.PARTIAL_WAKE_LOCK,
                    "pawstash:DownloadWakeLock"
                ).apply {
                    setReferenceCounted(false)
                    acquire(30 * 60 * 1000L) // 30 mins max safety limit
                }
                Log.d(TAG, "WakeLock acquired")
            }
        } catch (e: Throwable) {
            Log.e(TAG, "Error acquiring WakeLock", e)
        }
    }

    private fun releaseWakeLock() {
        try {
            if (wakeLock?.isHeld == true) {
                wakeLock?.release()
                Log.d(TAG, "WakeLock released")
            }
            wakeLock = null
        } catch (e: Throwable) {
            Log.e(TAG, "Error releasing WakeLock", e)
        }
    }

    private fun shutdownService() {
        isServiceRunning = false
        if (activeServiceInstance == this) {
            activeServiceInstance = null
        }
        releaseWakeLock()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        stopSelf()
        Log.d(TAG, "DownloadForegroundService stopped")
    }

    override fun onDestroy() {
        super.onDestroy()
        shutdownService()
    }
}
