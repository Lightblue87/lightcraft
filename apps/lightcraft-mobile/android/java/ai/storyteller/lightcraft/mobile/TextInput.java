package ai.storyteller.lightcraft.mobile;

import android.content.Context;
import android.text.Editable;
import android.text.InputType;
import android.text.Selection;
import android.view.KeyEvent;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import java.util.ArrayList;
import java.util.List;

/**
 * The soft keyboard's text editor for egui text fields. NativeActivity's own view has no input
 * connection, so the keyboard falls back to key events: no composing (CJK, Korean, Indic
 * scripts), autocorrect or swipe typing. This invisible view takes the keyboard while an egui text
 * field has focus; its input connection keeps a shadow copy of what was typed (the keyboard reads
 * it for suggestions) and queues every edit for the Rust shell, which polls {@link #take} and turns
 * them into egui IME events:
 *
 * <ul>
 *   <li>{@code P<text>}: the composing (pre-edit) text is now {@code text};
 *   <li>{@code C<text>}: commit {@code text} (in place of the composing text);
 *   <li>{@code D<before>,<after>}: delete that many characters (code points) around the cursor.
 * </ul>
 *
 * Key events the keyboard sends (Delete with nothing left to delete, Enter) go to the native input
 * queue like hardware keys.
 */
final class TextInput extends View {
    /** Called (on the UI thread) whenever edits were queued: the Rust side wakes its event loop. */
    interface Wake {
        void wake();
    }

    private final Object lock = new Object();
    private final List<String> queue = new ArrayList<>();
    private final Wake wake;

    TextInput(Context context, Wake wake) {
        super(context);
        this.wake = wake;
        setFocusable(true);
        setFocusableInTouchMode(true);
    }

    /** The edits queued since the last call, oldest first. */
    String[] take() {
        synchronized (lock) {
            String[] out = queue.toArray(new String[0]);
            queue.clear();
            return out;
        }
    }

    void clear() {
        synchronized (lock) {
            queue.clear();
        }
    }

    private void push(String edit) {
        synchronized (lock) {
            queue.add(edit);
        }
        wake.wake();
    }

    @Override
    public boolean onCheckIsTextEditor() {
        return true;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo info) {
        // plain text with suggestions; Enter is a key (egui decides: a new line, or done)
        info.inputType = InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_AUTO_CORRECT;
        info.imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN | EditorInfo.IME_FLAG_NO_EXTRACT_UI | EditorInfo.IME_FLAG_NO_ENTER_ACTION
                | EditorInfo.IME_ACTION_NONE;
        info.initialSelStart = 0;
        info.initialSelEnd = 0;
        return new Connection();
    }

    /** Code points in {@code s} from {@code a} to {@code b} (UTF-16 indices, clamped). */
    private static int codePoints(CharSequence s, int a, int b) {
        a = Math.max(0, Math.min(a, s.length()));
        b = Math.max(a, Math.min(b, s.length()));
        return Character.codePointCount(s, a, b);
    }

    private final class Connection extends BaseInputConnection {
        Connection() {
            super(TextInput.this, true);
        }

        /**
         * Tell the keyboard where the cursor and the composing text are now: it relies on these
         * reports (a text view's) to compose words and to replace them with suggestions.
         */
        private boolean report(boolean ok) {
            Editable e = getEditable();
            InputMethodManager imm = (InputMethodManager) getContext().getSystemService(Context.INPUT_METHOD_SERVICE);
            if (imm != null) {
                imm.updateSelection(TextInput.this, Selection.getSelectionStart(e), Selection.getSelectionEnd(e), getComposingSpanStart(e),
                        getComposingSpanEnd(e));
            }
            return ok;
        }

        private String composing() {
            Editable e = getEditable();
            int a = getComposingSpanStart(e), b = getComposingSpanEnd(e);
            return a < 0 || b < 0 ? "" : e.subSequence(Math.min(a, b), Math.max(a, b)).toString();
        }

        @Override
        public boolean setComposingText(CharSequence text, int newCursorPosition) {
            push("P" + text);
            return report(super.setComposingText(text, newCursorPosition));
        }

        @Override
        public boolean commitText(CharSequence text, int newCursorPosition) {
            push("C" + text);
            return report(super.commitText(text, newCursorPosition));
        }

        @Override
        public boolean finishComposingText() {
            String c = composing();
            if (!c.isEmpty()) push("C" + c);
            return report(super.finishComposingText());
        }

        @Override
        public boolean setComposingRegion(int start, int end) {
            // the keyboard re-opens a word already typed (e.g. backspacing into it): it becomes the
            // composing text again
            Editable e = getEditable();
            int a = Math.max(0, Math.min(Math.min(start, end), e.length())), b = Math.max(a, Math.min(Math.max(start, end), e.length()));
            int cursor = Selection.getSelectionEnd(e);
            if (b > a && cursor >= 0) {
                String word = e.subSequence(a, b).toString();
                int before = cursor > a ? codePoints(e, a, cursor) : 0;
                int after = b > cursor ? codePoints(e, cursor, b) : 0;
                push("D" + before + "," + after);
                push("P" + word);
            }
            return report(super.setComposingRegion(start, end));
        }

        @Override
        public boolean deleteSurroundingText(int before, int after) {
            // UTF-16 units → code points, over the shadow copy (text from before the field got
            // focus isn't in it: one unit each)
            Editable e = getEditable();
            int cursor = Math.max(0, Selection.getSelectionStart(e));
            int end = Math.max(cursor, Selection.getSelectionEnd(e));
            int inBefore = Math.min(before, cursor), inAfter = Math.min(after, e.length() - end);
            int b = codePoints(e, cursor - inBefore, cursor) + (before - inBefore);
            int a = codePoints(e, end, end + inAfter) + (after - inAfter);
            if (b > 0 || a > 0) push("D" + b + "," + a);
            return report(super.deleteSurroundingText(before, after));
        }

        @Override
        public boolean deleteSurroundingTextInCodePoints(int before, int after) {
            if (before > 0 || after > 0) push("D" + before + "," + after);
            return report(super.deleteSurroundingTextInCodePoints(before, after));
        }

        @Override
        public boolean setSelection(int start, int end) {
            // (egui keeps its own cursor: the keyboard moving it isn't passed on)
            return report(super.setSelection(start, end));
        }

        @Override
        public boolean sendKeyEvent(KeyEvent event) {
            // the key goes to the native input queue (egui applies it); the shadow copy follows the
            // deleting keys, so what the keyboard reads back stays what egui shows
            if (event.getAction() == KeyEvent.ACTION_DOWN) {
                int code = event.getKeyCode();
                if (code == KeyEvent.KEYCODE_DEL || code == KeyEvent.KEYCODE_FORWARD_DEL) {
                    Editable e = getEditable();
                    int a = Math.max(0, Selection.getSelectionStart(e)), b = Math.max(a, Selection.getSelectionEnd(e));
                    if (a == b) {
                        if (code == KeyEvent.KEYCODE_DEL && a > 0) a = Character.offsetByCodePoints(e, a, -1);
                        else if (code == KeyEvent.KEYCODE_FORWARD_DEL && b < e.length()) b = Character.offsetByCodePoints(e, b, 1);
                    }
                    if (b > a) e.delete(a, b);
                    report(true);
                }
            }
            return super.sendKeyEvent(event);
        }

        @Override
        public boolean performEditorAction(int action) {
            // (no action is offered, but some keyboards send one for Enter anyway)
            long t = android.os.SystemClock.uptimeMillis();
            sendKeyEvent(new KeyEvent(t, t, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER, 0));
            sendKeyEvent(new KeyEvent(t, t, KeyEvent.ACTION_UP, KeyEvent.KEYCODE_ENTER, 0));
            return true;
        }
    }
}
