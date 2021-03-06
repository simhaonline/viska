package viska.android

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.compose.foundation.Image
import androidx.compose.foundation.Text
import androidx.compose.foundation.layout.preferredSize
import androidx.compose.foundation.lazy.LazyColumnFor
import androidx.compose.material.ListItem
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Scaffold
import androidx.compose.material.TopAppBar
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.setContent
import androidx.compose.ui.unit.dp
import dagger.hilt.android.AndroidEntryPoint
import java.lang.IllegalArgumentException
import javax.inject.Inject
import viska.couchbase.AndroidChatroomRepository
import viska.couchbase.AndroidMessageRepository
import viska.database.Database.Message
import viska.database.displayId

@AndroidEntryPoint
class ChatroomActivity : InstanceActivity() {

  @Inject lateinit var chatroomRepository: AndroidChatroomRepository
  @Inject lateinit var messageRepository: AndroidMessageRepository

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    cancelIfNoActiveAccount()

    val chatroomId = chatroomId

    setContent {
      MaterialTheme {
        val chatroom by chatroomRepository.watchChatroom(chatroomId).collectAsState()
        if (chatroom == null) {
          finish()
          return@MaterialTheme
        }

        val messages by messageRepository.watchChatroomMessages(chatroomId).collectAsState()

        Scaffold(topBar = { TopAppBar(title = { Text(text = chatroom?.inner?.name ?: "") }) }) {
        _ ->
          LazyColumnFor(messages) { MessageItem(it) }
        }
      }
    }
  }

  private val chatroomId: String
    get() {
      val uriPath = intent.data?.pathSegments ?: emptyList()
      if (uriPath.size != 2 && "chatroom" != uriPath[0]) {
        throw IllegalArgumentException("Bad chatroom URI")
      }
      return uriPath[1]
    }

  companion object {

    fun start(source: Context, chatroomId: ByteArray) {
      // Will be like:
      // viska://chatroom/87956192a8143476909113cda0d4077e092e26e10cc7dac43e68f694ea68a036
      val uri =
          Uri.Builder()
              .scheme("viska")
              .authority("chatroom")
              .appendPath(chatroomId.displayId())
              .build()
      val intent = Intent(source, ChatroomActivity::class.java)
      intent.data = uri
      source.startActivity(intent)
    }
  }
}

@Composable
private fun MessageItem(message: Message) {
  ListItem(
      icon = { Image(asset = Icons.Default.Person, Modifier.preferredSize(48.dp)) },
      text = { Text(text = message.inner.content) })
}
