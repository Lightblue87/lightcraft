package ai.storyteller.lightcraft.mobile;

import android.app.NativeActivity;
import android.content.ClipData;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Bitmap;
import android.graphics.ImageDecoder;
import android.net.Uri;
import android.os.Build;
import android.provider.MediaStore;
import android.provider.OpenableColumns;
import android.view.ViewGroup;
import android.view.inputmethod.InputMethodManager;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.List;

/**
 * NativeActivity plus the photo picker, the share sheet and the soft keyboard's text input
 * ({@link TextInput}). NativeActivity can't host the picker: it never sees
 * onActivityResult. The Rust shell calls {@link #pickPhotos} over JNI and polls
 * {@link #takePicked}; each chosen photo is copied into {@code dir} on a background thread first.
 * HEIC/HEIF (which LightCraft can't decode yet) is converted to JPEG.
 */
public class LightCraftActivity extends NativeActivity {
    private static final int PICK = 0x4c43;
    private final Object lock = new Object();
    private final List<String[]> ready = new ArrayList<>();
    private String dir;

    // (anonymous classes, not lambdas: android.jar has no LambdaMetafactory.metafactory for javac)

    /** Show the system photo picker; picked photos are copied into {@code dir}. */
    public void pickPhotos(String dir) {
        this.dir = dir;
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
            Intent intent;
            if (Build.VERSION.SDK_INT >= 33) {
                intent = new Intent(MediaStore.ACTION_PICK_IMAGES);
                intent.putExtra(MediaStore.EXTRA_PICK_IMAGES_MAX, MediaStore.getPickImagesMaxLimit());
            } else {
                intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                intent.setType("image/*");
                intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
            }
            startActivityForResult(intent, PICK);
            }
        });
    }

    /** Offer exported files (newline-separated paths under the data directory) in the share sheet. */
    public void share(String joined) {
        final ArrayList<Uri> uris = new ArrayList<>();
        String type = null;
        for (String path : joined.split("\n")) {
            if (path.isEmpty()) continue;
            Uri uri = FilesProvider.uriFor(new File(path));
            uris.add(uri);
            String t = getContentResolver().getType(uri);
            type = type == null || type.equals(t) ? t : "image/*";
        }
        if (uris.isEmpty()) return;
        final Intent send = new Intent(uris.size() == 1 ? Intent.ACTION_SEND : Intent.ACTION_SEND_MULTIPLE);
        send.setType(type);
        if (uris.size() == 1) {
            send.putExtra(Intent.EXTRA_STREAM, uris.get(0));
        } else {
            send.putParcelableArrayListExtra(Intent.EXTRA_STREAM, uris);
        }
        // the grant covers every URI in the clip data
        ClipData clip = ClipData.newRawUri(null, uris.get(0));
        for (int i = 1; i < uris.size(); i++) clip.addItem(new ClipData.Item(uris.get(i)));
        send.setClipData(clip);
        send.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                startActivity(Intent.createChooser(send, null));
            }
        });
    }

    private volatile TextInput textInput;

    /** Wakes the Rust event loop (registered by the shell over JNI; see {@code android_jni}). */
    private native void nativeWake();

    /**
     * An egui text field got focus: the soft keyboard types into {@link TextInput} (composing
     * text, suggestions), starting from the field's text and selection (code points), set up for
     * its purpose ({@link TextInput#NORMAL}, {@link TextInput#PASSWORD}, {@link TextInput#TERMINAL},
     * {@link TextInput#NUMBER}).
     */
    public void startTextInput(final String text, final int selStart, final int selEnd, final int purpose) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (textInput == null) {
                    textInput = new TextInput(LightCraftActivity.this, new TextInput.Wake() {
                        @Override
                        public void wake() {
                            try {
                                nativeWake();
                            } catch (Throwable ignored) {
                                // not registered yet: the shell polls anyway
                            }
                        }
                    });
                    addContentView(textInput, new ViewGroup.LayoutParams(1, 1));
                }
                TextInput t = textInput;
                t.reset(text, selStart, selEnd, purpose);
                t.setFocusable(true);
                t.setFocusableInTouchMode(true);
                t.requestFocus();
                InputMethodManager imm = (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
                imm.restartInput(t);
                imm.showSoftInput(t, 0);
            }
        });
    }

    /**
     * The focused field's text and selection (code points) changed other than by the keyboard's
     * edits: the keyboard's copy follows ({@link TextInput#sync}).
     */
    public void syncTextInput(final String text, final int selStart, final int selEnd) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                TextInput t = textInput;
                if (t != null) t.sync(text, selStart, selEnd);
            }
        });
    }

    /** No egui text field has focus any more: the keyboard goes away. */
    public void stopTextInput() {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                TextInput t = textInput;
                if (t == null) return;
                InputMethodManager imm = (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
                imm.hideSoftInputFromWindow(t.getWindowToken(), 0);
                // not focusable, or focus would come straight back to it
                t.setFocusable(false);
                t.clearFocus();
                t.clear();
            }
        });
    }

    /** The keyboard's edits since the last call (see {@link TextInput}). */
    public String[] takeTextInput() {
        TextInput t = textInput;
        return t == null ? new String[0] : t.take();
    }

    private volatile int trimLevel;

    @Override
    public void onTrimMemory(int level) {
        super.onTrimMemory(level);
        // running low while in front, or anything after the UI is hidden: shrink caches
        if (level >= TRIM_MEMORY_RUNNING_LOW) trimLevel = Math.max(trimLevel, level);
    }

    @Override
    public void onLowMemory() {
        super.onLowMemory();
        trimLevel = Math.max(trimLevel, TRIM_MEMORY_COMPLETE);
    }

    /** The strongest memory trim level since the last call (0: none), for the Rust side. */
    public int takeTrimLevel() {
        int level = trimLevel;
        trimLevel = 0;
        return level;
    }

    /** The paths of one finished selection (empty when cancelled), or null; oldest first. */
    public String[] takePicked() {
        synchronized (lock) {
            return ready.isEmpty() ? null : ready.remove(0);
        }
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != PICK) return;
        if (result != RESULT_OK || data == null) {
            finished(new ArrayList<>()); // cancelled: the shell stops waiting
            return;
        }
        List<Uri> uris = new ArrayList<>();
        ClipData clip = data.getClipData();
        if (clip != null) {
            for (int i = 0; i < clip.getItemCount(); i++) uris.add(clip.getItemAt(i).getUri());
        } else if (data.getData() != null) {
            uris.add(data.getData());
        }
        final String target = dir;
        final List<Uri> picked = uris;
        new Thread(new Runnable() {
            @Override
            public void run() {
            List<String> paths = new ArrayList<>();
            for (Uri uri : picked) {
                try {
                    paths.add(copy(uri, target));
                } catch (Exception e) {
                    android.util.Log.w("LightCraft", "copying " + uri + ": " + e);
                }
            }
            finished(paths);
            }
        }).start();
    }

    private void finished(List<String> paths) {
        synchronized (lock) {
            ready.add(paths.toArray(new String[0]));
        }
    }

    private String copy(Uri uri, String target) throws Exception {
        File out = new File(target);
        out.mkdirs();
        String name = displayName(uri);
        String type = getContentResolver().getType(uri);
        boolean heif = type != null && (type.equals("image/heic") || type.equals("image/heif"));
        if (heif) name = name.replaceAll("\\.(?i)(heic|heif)$", "") + ".jpg";
        File file = free(out, name);
        if (heif) {
            Bitmap bitmap = ImageDecoder.decodeBitmap(ImageDecoder.createSource(getContentResolver(), uri));
            try (OutputStream os = new FileOutputStream(file)) {
                bitmap.compress(Bitmap.CompressFormat.JPEG, 95, os);
            }
        } else {
            try (InputStream in = getContentResolver().openInputStream(uri); OutputStream os = new FileOutputStream(file)) {
                byte[] buf = new byte[1 << 16];
                for (int n; (n = in.read(buf)) > 0; ) os.write(buf, 0, n);
            }
        }
        return file.getAbsolutePath();
    }

    private String displayName(Uri uri) {
        try (Cursor c = getContentResolver().query(uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (c != null && c.moveToFirst() && c.getString(0) != null) return c.getString(0);
        } catch (Exception ignored) {
        }
        return "photo-" + System.currentTimeMillis() + ".jpg";
    }

    /** {@code name} in {@code dir}, numbered when taken (picking a photo again keeps the first copy). */
    private static File free(File dir, String name) {
        File f = new File(dir, name);
        int dot = name.lastIndexOf('.');
        String stem = dot > 0 ? name.substring(0, dot) : name, ext = dot > 0 ? name.substring(dot) : "";
        for (int n = 1; f.exists(); n++) f = new File(dir, stem + "-" + n + ext);
        return f;
    }
}
