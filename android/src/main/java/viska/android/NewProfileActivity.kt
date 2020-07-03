package viska.android

import android.content.Intent
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.Composable
import androidx.ui.core.Alignment
import androidx.ui.core.Modifier
import androidx.ui.core.setContent
import androidx.ui.foundation.Text
import androidx.ui.layout.Arrangement
import androidx.ui.layout.Column
import androidx.ui.layout.fillMaxSize
import androidx.ui.material.Button
import androidx.ui.tooling.preview.Preview
import viska.database.createNewProfile

class NewProfileActivity : AppCompatActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    setContent { Ui() }
  }

  @Composable
  @Preview
  private fun Ui() {
    Column(
        verticalArrangement = Arrangement.SpaceEvenly,
        horizontalGravity = Alignment.CenterHorizontally,
        modifier = Modifier.fillMaxSize()) {
      Button(onClick = this@NewProfileActivity::newAccount) {
        Text(getString(R.string.new_account))
      }
      if (BuildConfig.DEBUG) {
        Button(onClick = this@NewProfileActivity::newMockProfile) {
          Text(getString(R.string.new_mock_profile))
        }
      }
    }
  }

  private fun newAccount() {
    val database = viska.database.open()
    database.createNewProfile()
    startActivity(Intent(this, MainActivity::class.java))
    finish()
  }

  private fun newMockProfile() {
    // Nothing!
  }
}
