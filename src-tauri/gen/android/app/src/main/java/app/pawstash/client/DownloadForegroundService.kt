package app.pawstash.client

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat

class DownloadForegroundService : Service() {

    companion object {
        private const val TAG = "PawstashService"
        const val CHANNEL_DOWNLOADS_ID = "pawstash_downloads"
        const val CHANNEL_COMPLETED_ID = "pawstash_download_completed"
        const val FOREGROUND_NOTIFICATION_ID = 1001
        private const val BASE_COMPLETED_NOTIFICATION_ID = 2000

        private var completedNotificationCount = 0
        private var wakeLock: PowerManager.WakeLock? = null
        private var lastUpdateTime = 0L

        @Volatile
        var isServiceRunning = false
            private set

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

        fun notifyCompleted(
            context: Context,
            service: String,
            creatorId: String,
            postId: String,
            filename: String,
            title: String,
            mediaCount: Int
        ) {
            try {
                ensureNotificationChannels(context)
                val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

                val intent = Intent(context, MainActivity::class.java).apply {
                    flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
                    putExtra("deep_link_action", "open_post")
                    putExtra("deep_link_service", service)
                    putExtra("deep_link_creator_id", creatorId)
                    putExtra("deep_link_post_id", postId)
                }

                val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                } else {
                    PendingIntent.FLAG_UPDATE_CURRENT
                }

                val notifId = BASE_COMPLETED_NOTIFICATION_ID + (++completedNotificationCount % 100)
                val pendingIntent = PendingIntent.getActivity(context, notifId, intent, flags)

                val displayTitle = if (title.isNotBlank()) title else filename
                val subtitle = if (mediaCount > 1) {
                    "Downloaded $mediaCount files"
                } else {
                    "Downloaded: $filename"
                }

                val notification = NotificationCompat.Builder(context, CHANNEL_COMPLETED_ID)
                    .setSmallIcon(R.mipmap.ic_launcher)
                    .setContentTitle("Download Complete")
                    .setContentText(displayTitle)
                    .setSubText(subtitle)
                    .setAutoCancel(true)
                    .setPriority(NotificationCompat.PRIORITY_DEFAULT)
                    .setContentIntent(pendingIntent)
                    .build()

                manager.notify(notifId, notification)
                Log.d(TAG, "Completed notification posted: $displayTitle")
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
                        NotificationManager.IMPORTANCE_DEFAULT
                    ).apply {
                        description = "Notifications when downloads finish"
                        setShowBadge(true)
                        enableVibration(true)
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
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        ensureNotificationChannels(this)
        acquireWakeLock()
        activeServiceInstance = this
        isServiceRunning = true

        val initialNotification = NotificationCompat.Builder(this, CHANNEL_DOWNLOADS_ID)
            .setSmallIcon(R.mipmap.ic_launcher)
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
            "ACTION_UPDATE_PROGRESS" -> {
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
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("deep_link_action", "open_downloads")
        }

        val pendingFlags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        } else {
            PendingIntent.FLAG_UPDATE_CURRENT
        }
        val pendingIntent = PendingIntent.getActivity(this, FOREGROUND_NOTIFICATION_ID, openIntent, pendingFlags)

        val notification = NotificationCompat.Builder(this, CHANNEL_DOWNLOADS_ID)
            .setSmallIcon(R.mipmap.ic_launcher)
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
