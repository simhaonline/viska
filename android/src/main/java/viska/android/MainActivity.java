package viska.android;

import android.content.DialogInterface;
import android.content.Intent;
import android.os.Bundle;
import android.view.MenuItem;
import android.view.View;
import android.widget.TextView;
import androidx.core.view.GravityCompat;
import androidx.drawerlayout.widget.DrawerLayout;
import androidx.lifecycle.MutableLiveData;
import androidx.lifecycle.ViewModelProvider;
import com.google.android.material.appbar.MaterialToolbar;
import com.google.android.material.dialog.MaterialAlertDialogBuilder;
import com.google.android.material.navigation.NavigationView;
import io.reactivex.disposables.CompositeDisposable;
import io.reactivex.disposables.Disposable;
import viska.database.Database;

public class MainActivity extends Activity {

  public static class MainViewModel extends androidx.lifecycle.ViewModel {

    final MutableLiveData<Screen> screen = new MutableLiveData<>();

    public MainViewModel() {
      screen.setValue(Screen.CHATROOMS);
    }
  }

  public enum Screen {
    CHATROOMS,
    ROSTER
  }

  private ViskaService.Connection viska;
  private Intent viskaIntent;
  private Database db;
  private MainViewModel model;
  private CompositeDisposable subscriptions;
  private Disposable vcardSubscription;

  @Override
  protected void onCreate(Bundle savedInstanceState) {
    super.onCreate(savedInstanceState);

    setContentView(R.layout.main);
    model = new ViewModelProvider(this).get(MainViewModel.class);
    viskaIntent = new Intent(this, ViskaService.class);

    final MaterialToolbar actionBar = findViewById(R.id.action_bar);
    final DrawerLayout drawerLayout = findViewById(R.id.drawer_layout);
    setSupportActionBar(actionBar);
    actionBar.setNavigationOnClickListener(it -> drawerLayout.openDrawer(GravityCompat.START));

    final View fab = findViewById(R.id.fab);
    fab.setOnClickListener(this::onFabClicked);

    final NavigationView drawer = findViewById(R.id.drawer);
    drawer.setNavigationItemSelectedListener(this::onNavigationItemSelected);

    model.screen.observe(this, this::changeScreen);
  }

  @Override
  protected void onStart() {
    super.onStart();

    db = ((Application) getApplication()).getDatabase();
    if (db.isEmpty()) {
      startActivity(new Intent(this, NewProfileActivity.class));
      finish();
      return;
    }

    startForegroundService(viskaIntent);
    viska = new ViskaService.Connection();
    bindService(viskaIntent, viska, 0);

    subscriptions = new CompositeDisposable();

    final NavigationView drawer = findViewById(R.id.drawer);

    final TextView description = drawer.getHeaderView(0).findViewById(R.id.description);
    final TextView name = drawer.getHeaderView(0).findViewById(R.id.name);
    subscriptions.add(db.getAccountId().forEach(id -> {
      description.setText(id);

      if (vcardSubscription != null) {
        subscriptions.remove(vcardSubscription);
      }
      final Disposable newVcardSubscription = db.getVcard(id).forEach(vcard -> name.setText(vcard.name));
      subscriptions.add(newVcardSubscription);
      vcardSubscription = newVcardSubscription;
    }));


  }

  @Override
  protected void onStop() {
    super.onStop();
    if (viska != null) {
      unbindService(viska);
      viska = null;
    }
    if (db != null) {
      db.close();
    }
    if (subscriptions != null) {
      subscriptions.dispose();
    }
  }

  private boolean onNavigationItemSelected(final MenuItem item) {
    switch (item.getItemId()) {
      case R.id.chatrooms:
        item.setChecked(true);
        model.screen.setValue(Screen.CHATROOMS);
        return true;
      case R.id.roster:
        item.setChecked(true);
        model.screen.setValue(Screen.ROSTER);
        return true;
      case R.id.exit:
        askExit();
        return true;
      default:
        return false;
    }
  }

  private void changeScreen(final Screen screen) {
    final NavigationView drawer = findViewById(R.id.drawer);
    final MenuItem drawerMenuChatrooms = drawer.getMenu().findItem(R.id.chatrooms);
    final MenuItem drawerMenuRoster = drawer.getMenu().findItem(R.id.roster);
    switch (screen) {
      case CHATROOMS:
        drawerMenuChatrooms.setChecked(true);
        getSupportActionBar().setTitle(R.string.title_chatrooms);
        // Change list adapter
        break;
      case ROSTER:
        drawerMenuRoster.setChecked(true);
        getSupportActionBar().setTitle(R.string.title_roster);
        // Change list adapter
        break;
      default:
        return;
    }

    final DrawerLayout drawerLayout = findViewById(R.id.drawer_layout);
    drawerLayout.closeDrawers();
  }

  private void askExit() {
    final DialogInterface.OnClickListener listener = (dialog, which) -> {
      if (which != DialogInterface.BUTTON_POSITIVE) {
        return;
      }
      stopService(viskaIntent);
      finish();
    };

    new MaterialAlertDialogBuilder(this)
        .setTitle(R.string.dialog_exit_title)
        .setMessage(R.string.dialog_exit_text)
        .setPositiveButton(R.string.title_yes, listener)
        .setNegativeButton(R.string.title_no, listener)
        .create()
        .show();
  }

  private void onFabClicked(final View view) {
  }
}
