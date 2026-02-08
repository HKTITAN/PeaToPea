package dev.peapod.android

import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import dev.peapod.android.databinding.ActivityMainBinding

/**
 * Main screen: Enable PeaPod (starts VPN with system consent), status (.tasks/03-android §2.1.3, §6.1, §6.3).
 */
class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding

    private val vpnPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == RESULT_OK) {
            startVpn()
        } else {
            binding.status.text = getString(R.string.status_consent_denied)
        }
    }

    private val localNetworkPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            proceedToVpnPrepare()
        } else {
            showPermissionDeniedOrOpenSettings()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)
        binding.buttonEnable.setOnClickListener { onEnableClicked() }
        binding.buttonSettings.setOnClickListener { startActivity(Intent(this, SettingsActivity::class.java)) }
        binding.status.text = ""
        binding.podStatus.text = ""
        if (!PeaPodPreferences.hasSeenFirstRun(this)) {
            showFirstRunDialog()
        }
    }

    override fun onResume() {
        super.onResume()
        updatePodStatus()
    }

    private fun updatePodStatus() {
        if (PeaPodVpnService.vpnActive) {
            binding.status.text = getString(R.string.peapod_active)
            binding.podStatus.text = if (PeaPodVpnService.peerCountForUi <= 0) {
                getString(R.string.no_peers_nearby)
            } else {
                getString(R.string.peapod_pod_devices, PeaPodVpnService.peerCountForUi)
            }
        } else {
            binding.status.text = getString(R.string.peapod_off)
            binding.podStatus.text = getString(R.string.no_peers_nearby)
        }
    }

    private fun showFirstRunDialog() {
        AlertDialog.Builder(this)
            .setTitle(R.string.first_run_title)
            .setMessage(R.string.first_run_message)
            .setPositiveButton(android.R.string.ok) { _, _ ->
                PeaPodPreferences.setFirstRunSeen(this)
            }
            .setCancelable(true)
            .show()
    }

    private fun onEnableClicked() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val perm = android.Manifest.permission.NEARBY_WIFI_DEVICES
            when (ContextCompat.checkSelfPermission(this, perm)) {
                android.content.pm.PackageManager.PERMISSION_GRANTED -> proceedToVpnPrepare()
                else -> localNetworkPermissionLauncher.launch(perm)
            }
        } else {
            proceedToVpnPrepare()
        }
    }

    private fun proceedToVpnPrepare() {
        val intent = VpnService.prepare(this)
        if (intent != null) {
            vpnPermissionLauncher.launch(intent)
        } else {
            startVpn()
        }
    }

    private fun showPermissionDeniedOrOpenSettings() {
        AlertDialog.Builder(this)
            .setMessage(getString(R.string.permission_local_network_denied) + " " + getString(R.string.permission_denied_settings_hint))
            .setPositiveButton(R.string.permission_open_settings) { _, _ ->
                val i = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
                i.data = android.net.Uri.parse("package:$packageName")
                startActivity(i)
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun startVpn() {
        val intent = Intent(this, PeaPodVpnService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
        updatePodStatus()
    }
}
