package com.actyx.os.android

import android.content.Context
import android.content.Intent
import android.content.Intent.FLAG_ACTIVITY_NEW_TASK
import android.net.Uri
import arrow.core.Either
import arrow.core.Option
import com.actyx.os.android.activity.MainActivity
import com.actyx.os.android.activity.WebappActivity
import com.actyx.os.android.api.RestServer
import com.actyx.os.android.service.BackgroundServices
import com.actyx.os.android.util.Manifest
import io.reactivex.Observable
import io.reactivex.subjects.BehaviorSubject
import java.io.File
import java.io.FileInputStream
import java.io.InputStream

/**
 * Repository of deployed ActyxOS Webapps.
 *
 * Structure:
 *    root
 *    └─ {appId}
 *       ├─ current // text file that contains the latest version
 *       └─ {version}
 *          ├─ ax-manifest.yml
 *          └─ {extracted app archive content}
 */
class AppRepository(extFilesDir: File, val ctx: Context) {

  private val tempDir = File(extFilesDir, "tmp")
  private val baseDir = File(extFilesDir, "apps")

  private val appsSubject = BehaviorSubject.create<List<AppInfo>>()

  init {
    if (!baseDir.exists()) baseDir.mkdir()
    if (!tempDir.exists()) tempDir.mkdir()
    appsSubject.onNext(appInfoList())
  }

  private fun appInfo(manifest: Manifest.ManifestDetails): AppInfo =
    AppInfo(
      manifest.id,
      manifest.version,
      manifest.name,
      iconFile(manifest).map { it.absolutePath }.orNull(),
      appUrl(manifest),
      loadSettingsSchema(manifest)
    )

  fun appInfo(appId: String): AppInfo? =
    currentAppDir(appId)?.let { current ->
      appInfo(manifest(current).manifest)
    }

  /**
   * returns a list of all currently deployed apps
   */
  fun appInfoList(): List<AppInfo> =
    baseDir.listFiles { f -> f.isDirectory }
      ?.map { it.name }
      ?.mapNotNull(::appInfo)
      ?: listOf()

  fun observe(): Observable<List<AppInfo>> = appsSubject

  /**
   * loads the specified resource from the dist directory of the latest version of the
   * specified appid
   */
  fun getAppResourceAsStream(appId: String, resourcePath: String): Pair<InputStream, Long> =
    currentDistDir(appId)?.let { distDir ->
      File(distDir, resourcePath).let {
        if (it.isFile)
          Pair(FileInputStream(it), it.length())
        else
          throw ResourceNotFoundException(
            "Resource for app \"$appId\" not found: \"$resourcePath\""
          )
      }
    } ?: throw ResourceNotFoundException("App not found: \"$appId\"")

  /**
   * gets the app icon path referenced in the manifest within the repo
   */
  private fun iconFile(manifest: Manifest.ManifestDetails): Option<File> =
    manifest.appIconPath.map { File(currentAppDir(manifest.id), it) }

  /**
   * creates the url under which the app is served
   */
  private fun appUrl(manifest: Manifest.ManifestDetails): Uri =
    if (manifest.main.startsWith("http"))
      Uri.parse(manifest.main) // FIXME rm
    else
      Uri.Builder()
        .scheme("http")
        .encodedAuthority("localhost:${RestServer.Port}")
        // .path("apps/${manifest.id}/${manifest.main}
        // FIXME: Remove, once we rewrite the build manifest properly
        .path("apps/${manifest.id}/${manifest.main.split("/").last()}")
        .build()

  private fun loadSettingsSchema(manifest: Manifest.ManifestDetails): String {
    val schemaFile = File(currentAppDir(manifest.id), manifest.settingsSchema)
    return schemaFile.readText()
  }

  /**
   * base app directory that contains the version directories
   */
  private fun appBaseDir(id: String): File =
    File(baseDir, id)

  /**
   * version specific app directory
   */
  private fun appVersionDir(appDir: File, version: String): File =
    File(appDir, version)

  /**
   * directory that contains the latest app version
   */
  private fun currentAppDir(appId: String): File? {
    val appDir = appBaseDir(appId)
    val current = currentFile(appId)
    return if (current.exists()) {
      val currentVersion = current.readText().trim()
      File(appDir, currentVersion)
    } else null
  }

  private fun currentFile(appId: String): File =
    File(appBaseDir(appId), "current")

  /**
   * loads the version-specific app manifest
   */
  private fun manifest(appVersionDir: File): Manifest =
    Manifest.load(File(appVersionDir, "ax-manifest.yml").readText())

  /**
   * directory referenced from the manifest that contains the distributed resources
   */
  private fun currentDistDir(appId: String): File? =
    currentAppDir(appId)?.let { current ->
      manifest(current).manifest.dist.let { File(current, it) }
    }

  fun startApp(appId: String): Either<String, Unit> =
    Option.fromNullable(appInfo(appId))
      .toEither { "Unknown application ID '$appId'" }
      .map { app ->
        val intent = Intent(ctx, WebappActivity::class.java).apply {
          flags = FLAG_ACTIVITY_NEW_TASK
          action = Intent.ACTION_VIEW
          setDataAndType(app.uri, MainActivity.ACTYXOS_APP_MIME_TYPE)
          putExtra(WebappActivity.EXTRA_SHORTCUT_APP_ID, app.id)
        }
        ctx.startActivity(intent)
      }

  fun stopApp(appId: String): Either<String, Unit> =
    Option.fromNullable(appInfo(appId))
      .toEither { "Unknown application ID '$appId'" }
      .map { app ->
        val intent = Intent(BackgroundServices.ACTION_APP_STOP_REQUESTED).apply {
          putExtra(BackgroundServices.EXTRA_APP_ID, app.id)
        }
        ctx.sendBroadcast(intent)
      }

  class ResourceNotFoundException(msg: String) : RuntimeException(msg)
}
