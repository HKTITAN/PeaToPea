package dev.peapod.android

import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.Bundle
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import dev.peapod.android.databinding.ActivityMainBinding

/**
 * Main screen: Enable PeaPod (starts VPN with system consent), status (.tasks/03-android §2.1.3, §6.1).
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

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)
        binding.buttonEnable.setOnClickListener { onEnableClicked() }
        binding.status.text = ""
        binding.podStatus.text = ""
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

    private fun onEnableClicked() {
        val intent = VpnService.prepare(this)
        if (intent != null) {
            vpnPermissionLauncher.launch(intent)
        } else {
            startVpn()
        }
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
