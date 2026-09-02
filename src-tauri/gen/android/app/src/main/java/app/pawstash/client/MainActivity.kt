package app.pawstash.client

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Environment
import android.provider.DocumentsContract
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import java.io.File

class MainActivity : TauriActivity() {
  companion object {
    @Volatile
    var instance: MainActivity? = null

    @Volatile
    private var pendingDeepLinkPayload: String? = null

    @JvmStatic
    external fun initAndroidContext(activity: MainActivity)

    @JvmStatic
    external fun onFolderPicked(path: String?)

    @JvmStatic
    external fun onDeepLinkReceived(json: String)

    @JvmStatic
    fun scanMediaFile(filePath: String) {
      val ctx = instance ?: return
      try {
        android.media.MediaScannerConnection.scanFile(
          ctx,
          arrayOf(filePath),
          null,
          null
        )
      } catch (e: Throwable) {
        e.printStackTrace()
      }
    }

    @JvmStatic
    fun updateDownloadNotification(
      activeCount: Int,
      totalCount: Int,
      downloadedBytes: Long,
      totalBytes: Long,
      speedBytesPerSec: Long,
      currentFilename: String
    ) {
      try {
        val ctx = instance ?: return
        DownloadForegroundService.updateProgress(
          ctx,
          activeCount,
          totalCount,
          downloadedBytes,
          totalBytes,
          speedBytesPerSec,
          currentFilename
        )
      } catch (e: Throwable) {
        android.util.Log.e("Pawstash", "updateDownloadNotification error", e)
      }
    }

    @JvmStatic
    fun notifyDownloadCompleted(
      service: String,
      creatorId: String,
      postId: String,
      filename: String,
      title: String,
      mediaCount: Int
    ) {
      try {
        val ctx = instance ?: return
        DownloadForegroundService.notifyCompleted(
          ctx,
          service,
          creatorId,
          postId,
          filename,
          title,
          mediaCount
        )
      } catch (e: Throwable) {
        android.util.Log.e("Pawstash", "notifyDownloadCompleted error", e)
      }
    }

    @JvmStatic
    fun stopDownloadNotification() {
      try {
        val ctx = instance ?: return
        DownloadForegroundService.stopService(ctx)
      } catch (e: Throwable) {
        android.util.Log.e("Pawstash", "stopDownloadNotification error", e)
      }
    }

    @JvmStatic
    fun getPendingDeepLink(): String? {
      val link = pendingDeepLinkPayload
      pendingDeepLinkPayload = null
      return link
    }
  }

  private val openDocumentTreeLauncher = registerForActivityResult(
    ActivityResultContracts.OpenDocumentTree()
  ) { uri: Uri? ->
    val path = uri?.let { resolveTreeUriToPath(it) }
    onFolderPicked(path)
  }

  private val requestNotificationPermissionLauncher = registerForActivityResult(
    ActivityResultContracts.RequestPermission()
  ) { _ ->
    // Notification permission result
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    instance = this
    try {
      initAndroidContext(this)
      handleDeepLinkIntent(intent)
      if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
        if (checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) != android.content.pm.PackageManager.PERMISSION_GRANTED) {
          requestNotificationPermissionLauncher.launch(android.Manifest.permission.POST_NOTIFICATIONS)
        }
      }
    } catch (e: Throwable) {
      e.printStackTrace()
    }
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    setIntent(intent)
    handleDeepLinkIntent(intent)
  }

  private fun handleDeepLinkIntent(intent: Intent?) {
    if (intent == null) return

    // 1. Check for external deep links / universal links (e.g. pawstash://... or https://...)
    val dataUri = intent.dataString
    if (!dataUri.isNullOrBlank()) {
      pendingDeepLinkPayload = dataUri
      try {
        onDeepLinkReceived(dataUri)
      } catch (e: Throwable) {
        // If Tauri JNI isn't attached yet, will be retrieved on startup via getPendingDeepLink
      }
      return
    }

    // 2. Check for internal notification intent extras
    val action = intent.getStringExtra("deep_link_action") ?: return
    val service = intent.getStringExtra("deep_link_service") ?: ""
    val creatorId = intent.getStringExtra("deep_link_creator_id") ?: ""
    val postId = intent.getStringExtra("deep_link_post_id") ?: ""

    val json = """{"action":"$action","service":"$service","creatorId":"$creatorId","postId":"$postId"}"""
    pendingDeepLinkPayload = json
    try {
      onDeepLinkReceived(json)
    } catch (e: Throwable) {
      // If Tauri JNI isn't attached yet, will be retrieved on startup via getPendingDeepLink
    }
  }

  override fun onResume() {
    super.onResume()
    instance = this
    try {
      initAndroidContext(this)
    } catch (e: Throwable) {
      e.printStackTrace()
    }
  }

  override fun onDestroy() {
    super.onDestroy()
    if (instance == this) instance = null
  }

  fun launchFolderPicker() {
    runOnUiThread {
      openDocumentTreeLauncher.launch(null)
    }
  }

  fun installApk(apkPath: String) {
    runOnUiThread {
      try {
        val file = File(apkPath)
        if (!file.exists()) return@runOnUiThread
        val uri = androidx.core.content.FileProvider.getUriForFile(
          this,
          "$packageName.fileprovider",
          file
        )
        val intent = Intent(Intent.ACTION_VIEW).apply {
          setDataAndType(uri, "application/vnd.android.package-archive")
          addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        startActivity(intent)
      } catch (e: Throwable) {
        e.printStackTrace()
      }
    }
  }

  fun openFileInNativeViewer(filePath: String) {
    runOnUiThread {
      try {
        val file = File(filePath)
        if (!file.exists()) return@runOnUiThread
        val uri = androidx.core.content.FileProvider.getUriForFile(
          this,
          "$packageName.fileprovider",
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

        val intent = Intent(Intent.ACTION_VIEW).apply {
          setDataAndType(uri, mimeType)
          addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        startActivity(Intent.createChooser(intent, "Open with").apply {
          addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        })
      } catch (e: Throwable) {
        e.printStackTrace()
      }
    }
  }

  fun openFolderInFileManager(folderPath: String) {
    runOnUiThread {
      val cleanPath = folderPath.trimEnd('/')
      val folderFile = File(cleanPath)
      if (!folderFile.exists()) {
        try { folderFile.mkdirs() } catch (_: Throwable) {}
      }

      // Immediately scan existing files in the directory so MediaStore indexes them
      try {
        val filesToScan = mutableListOf<String>()
        filesToScan.add(cleanPath)
        folderFile.listFiles()?.forEach { f ->
          if (f.isFile) filesToScan.add(f.absolutePath)
        }
        if (filesToScan.isNotEmpty()) {
          android.media.MediaScannerConnection.scanFile(
            this,
            filesToScan.toTypedArray(),
            null,
            null
          )
        }
      } catch (_: Throwable) {}

      val relPath = if (cleanPath.startsWith("/storage/emulated/0/")) {
        cleanPath.removePrefix("/storage/emulated/0/")
      } else if (cleanPath.startsWith("/storage/emulated/0")) {
        cleanPath.removePrefix("/storage/emulated/0")
      } else {
        cleanPath
      }.trimStart('/')

      val docId = if (relPath.isNotEmpty()) "primary:$relPath" else "primary:Download"
      val docUri = try {
        DocumentsContract.buildDocumentUri("com.android.externalstorage.documents", docId)
      } catch (_: Throwable) { null }
      val treeUri = try {
        DocumentsContract.buildTreeDocumentUri("com.android.externalstorage.documents", docId)
      } catch (_: Throwable) { null }
      val docTreeUri = try {
        if (treeUri != null) DocumentsContract.buildDocumentUriUsingTree(treeUri, docId) else docUri
      } catch (_: Throwable) { docUri }

      val fileProviderUri = try {
        androidx.core.content.FileProvider.getUriForFile(
          this,
          "$packageName.fileprovider",
          folderFile
        )
      } catch (_: Throwable) { null }

      // 1. Explicit DocumentsUI / System Files Activity with target folder
      val documentsUiPackages = listOf("com.google.android.documentsui", "com.android.documentsui")
      for (pkg in documentsUiPackages) {
        if (docUri != null) {
          try {
            val intent = Intent(Intent.ACTION_VIEW).apply {
              setPackage(pkg)
              setDataAndType(docUri, DocumentsContract.Document.MIME_TYPE_DIR)
              if (treeUri != null) putExtra(DocumentsContract.EXTRA_INITIAL_URI, treeUri)
              if (treeUri != null) putExtra("android.provider.extra.INITIAL_URI", treeUri)
              addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_PREFIX_URI_PERMISSION)
            }
            startActivity(intent)
            return@runOnUiThread
          } catch (_: Throwable) {}
        }
      }

      // 2. Try android.provider.action.BROWSE with Document URI
      if (docUri != null) {
        try {
          val browseIntent = Intent("android.provider.action.BROWSE").apply {
            data = docUri
            if (treeUri != null) putExtra(DocumentsContract.EXTRA_INITIAL_URI, treeUri)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_PREFIX_URI_PERMISSION)
          }
          startActivity(browseIntent)
          return@runOnUiThread
        } catch (_: Throwable) {}
      }

      // 3. Try DocumentsUI with ACTION_VIEW on Document / Tree Uri
      if (docTreeUri != null || docUri != null) {
        try {
          val targetUri = docTreeUri ?: docUri
          val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(targetUri, DocumentsContract.Document.MIME_TYPE_DIR)
            if (treeUri != null) putExtra(DocumentsContract.EXTRA_INITIAL_URI, treeUri)
            if (treeUri != null) putExtra("android.provider.extra.INITIAL_URI", treeUri)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_PREFIX_URI_PERMISSION)
          }
          startActivity(intent)
          return@runOnUiThread
        } catch (_: Throwable) {}
      }

      // 4. Samsung MyFiles targeted folder action
      try {
        val samsungIntent = Intent("sec.intent.action.MYFILES_TO_FOLDER").apply {
          putExtra("FOLDERPATH", cleanPath)
          putExtra("samsung.myfiles.intent.extra.START_PATH", cleanPath)
          addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        startActivity(samsungIntent)
        return@runOnUiThread
      } catch (_: Throwable) {}

      // 5. Targeted OEM file manager packages with path extras & resource/folder
      val oemPackages = listOf(
        "com.hihonor.filemanager",
        "com.huawei.hidisk",
        "com.google.android.apps.nbu.files",
        "com.sec.android.app.myfiles",
        "com.mi.android.globalFileexplorer",
        "com.android.fileexplorer"
      )

      for (pkg in oemPackages) {
        try {
          val intent = Intent(Intent.ACTION_VIEW).apply {
            setPackage(pkg)
            if (fileProviderUri != null) {
              setDataAndType(fileProviderUri, "resource/folder")
            } else if (docUri != null) {
              setDataAndType(docUri, DocumentsContract.Document.MIME_TYPE_DIR)
            }
            putExtra("current_path", cleanPath)
            putExtra("root_path", cleanPath)
            putExtra("path", cleanPath)
            putExtra("folder", cleanPath)
            putExtra("FOLDERPATH", cleanPath)
            if (treeUri != null) putExtra(DocumentsContract.EXTRA_INITIAL_URI, treeUri)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_PREFIX_URI_PERMISSION)
          }
          startActivity(intent)
          return@runOnUiThread
        } catch (_: Throwable) {}
      }

      // 6. Generic ACTION_VIEW with FileProvider and resource/folder (Third party file managers)
      if (fileProviderUri != null) {
        for (mime in listOf("resource/folder", "vnd.android.document/directory")) {
          try {
            val intent = Intent(Intent.ACTION_VIEW).apply {
              setDataAndType(fileProviderUri, mime)
              putExtra("current_path", cleanPath)
              putExtra("path", cleanPath)
              putExtra("FOLDERPATH", cleanPath)
              addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
            }
            startActivity(intent)
            return@runOnUiThread
          } catch (_: Throwable) {}
        }
      }

      // 7. System Download Manager
      try {
        val intent = Intent(android.app.DownloadManager.ACTION_VIEW_DOWNLOADS).apply {
          addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        startActivity(intent)
      } catch (e: Throwable) {
        e.printStackTrace()
      }
    }
  }

  private fun resolveTreeUriToPath(treeUri: Uri): String {
    try {
      val docId = DocumentsContract.getTreeDocumentId(treeUri)
      val parts = docId.split(":")
      if (parts.isNotEmpty()) {
        val type = parts[0]
        val subPath = if (parts.size > 1) parts[1].trimEnd('/') else ""
        if ("primary".equals(type, ignoreCase = true)) {
          val root = Environment.getExternalStorageDirectory().absolutePath
          return if (subPath.isNotEmpty()) "$root/$subPath" else root
        } else {
          val externalFilesDirs = getExternalFilesDirs(null)
          for (f in externalFilesDirs) {
            if (f != null) {
              val absPath = f.absolutePath
              val index = absPath.indexOf("/Android/data")
              if (index > 0) {
                val root = absPath.substring(0, index)
                if (root.contains(type, ignoreCase = true)) {
                  return if (subPath.isNotEmpty()) "$root/$subPath" else root
                }
              }
            }
          }
          return if (subPath.isNotEmpty()) "/storage/$type/$subPath" else "/storage/$type"
        }
      }
    } catch (e: Exception) {
      e.printStackTrace()
    }
    return treeUri.toString()
  }
}
