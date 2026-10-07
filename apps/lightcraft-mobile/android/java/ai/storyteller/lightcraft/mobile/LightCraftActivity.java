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
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.List;

/**
 * NativeActivity plus the photo picker and the share sheet. NativeActivity can't host the picker: it never sees
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
