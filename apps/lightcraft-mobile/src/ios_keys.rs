//! Hardware keyboards on iPadOS (Magic Keyboard, Smart Keyboard, Bluetooth).
//!
//! winit 0.30's iOS view only implements `UIKeyInput` (typed text and ⌫, for the soft keyboard),
//! so no shortcut, arrow, Escape or modifier ever reached the app. While no text field has focus,
//! a hidden view of ours is the first responder instead: UIKit hands it the key presses
//! (`UIPress.key`: a HID usage and the ⌘⌥⌃⇧ flags), which become egui key events
//! ([`lightcraft_ui_egui::hid`]) on the next frame, with ⌘ as egui's command modifier. When egui
//! focuses a text field, winit's view takes the first responder back to show the keyboard and
//! receive text.
//!
//! Drop this once eframe moves to a winit with iOS key events (0.31 has `pressesBegan`).
#![allow(unsafe_code)] // Objective-C class definition and the window handle's UIView pointer

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{ClassType, MainThreadOnly, define_class, msg_send};
use objc2_core_foundation::CGRect;
use objc2_foundation::NSSet;
use objc2_ui_kit::{UIPress, UIPressesEvent, UIResponder, UIView};

thread_local! {
    // main thread only: UIKit calls the view there, and the shell's frames run there
    static VIEW: RefCell<Option<Retained<KeyView>>> = const { RefCell::new(None) };
    static QUEUE: RefCell<Vec<egui::Event>> = const { RefCell::new(Vec::new()) };
    static MODIFIERS: RefCell<egui::Modifiers> = const { RefCell::new(egui::Modifiers::NONE) };
    static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
}

define_class!(
    /// A 1 × 1 pt view that is the first responder while egui takes no text, for the key presses.
    #[unsafe(super(UIView, UIResponder, NSObject))]
    #[name = "LCKeyView"]
    #[thread_kind = MainThreadOnly]
    struct KeyView;

    impl KeyView {
        #[unsafe(method(canBecomeFirstResponder))]
        fn can_become_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(pressesBegan:withEvent:))]
        fn presses_began(&self, presses: &NSSet<UIPress>, _event: Option<&UIPressesEvent>) {
            queue(self, presses, true);
        }

        #[unsafe(method(pressesEnded:withEvent:))]
        fn presses_ended(&self, presses: &NSSet<UIPress>, _event: Option<&UIPressesEvent>) {
            queue(self, presses, false);
        }

        #[unsafe(method(pressesCancelled:withEvent:))]
        fn presses_cancelled(&self, presses: &NSSet<UIPress>, _event: Option<&UIPressesEvent>) {
            queue(self, presses, false);
        }
    }
);

/// HID usages of the modifier keys themselves (left / right ⌃ ⇧ ⌥ ⌘): their release clears them.
fn modifier_key(usage: i64, m: &mut egui::Modifiers) -> bool {
    match usage {
        0xE0 | 0xE4 => m.ctrl = false,
        0xE1 | 0xE5 => m.shift = false,
        0xE2 | 0xE6 => m.alt = false,
        0xE3 | 0xE7 => {
            m.command = false;
            m.mac_cmd = false;
        }
        _ => return false,
    }
    true
}

fn queue(view: &KeyView, presses: &NSSet<UIPress>, pressed: bool) {
    let mtm = view.mtm();
    for press in presses.iter() {
        let Some(key) = press.key(mtm) else { continue };
        let usage = key.keyCode().0 as i64;
        let mut modifiers = lightcraft_ui_egui::hid::apple_modifiers(key.modifierFlags().0 as i64);
        if !pressed {
            // a released modifier key may still be in the flags of its own release
            modifier_key(usage, &mut modifiers);
        }
        // egui follows the modifiers from these events
        let changed = MODIFIERS.with_borrow_mut(|m| std::mem::replace(m, modifiers) != modifiers);
        if changed {
            QUEUE.with_borrow_mut(|q| q.push(egui::Event::ModifiersChanged(modifiers)));
        }
        if let Some(k) = lightcraft_ui_egui::hid::key(usage) {
            QUEUE.with_borrow_mut(|q| q.push(egui::Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers }));
        }
    }
    CONTEXT.with_borrow(|c| {
        if let Some(c) = c {
            c.request_repaint();
        }
    });
}

/// Adds the key view to winit's view (from eframe's window handle), once.
pub fn install(handle: raw_window_handle::RawWindowHandle, ctx: &egui::Context) {
    let raw_window_handle::RawWindowHandle::UiKit(h) = handle else { return };
    CONTEXT.with_borrow_mut(|c| *c = Some(ctx.clone()));
    if VIEW.with_borrow(Option::is_some) {
        return;
    }
    // SAFETY: on iOS the window handle's `ui_view` is winit's UIView, alive while the window is
    let parent: &UIView = unsafe { h.ui_view.cast::<UIView>().as_ref() };
    let mtm = parent.mtm();
    let frame = CGRect::new(objc2_core_foundation::CGPoint::new(0.0, 0.0), objc2_core_foundation::CGSize::new(1.0, 1.0));
    // SAFETY: `initWithFrame:` is UIView's designated initializer; the class adds no ivars
    let view: Retained<KeyView> = unsafe { msg_send![KeyView::alloc(mtm), initWithFrame: frame] };
    view.setHidden(true);
    parent.addSubview(&view);
    VIEW.with_borrow_mut(|v| *v = Some(view));
    let _ = KeyView::class();
}

/// Moves the presses queued since the last frame into `raw`, and makes the key view the first
/// responder again when egui takes no text (`wants_text`: a text field has focus).
pub fn input(raw: &mut egui::RawInput, wants_text: bool) {
    QUEUE.with_borrow_mut(|q| raw.events.append(q));
    if wants_text {
        return;
    }
    VIEW.with_borrow(|v| {
        if let Some(v) = v
            && !v.isFirstResponder()
        {
            v.becomeFirstResponder();
        }
    });
}
