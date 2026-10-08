//! Apple Pencil hover and double tap, and the iPad pointer (trackpad, mouse) hovering.
//!
//! winit 0.30 on iOS reports touches only, so a Pencil held above the screen, or the pointer of a
//! Magic Keyboard's trackpad, never moved egui's pointer: no brush outline before painting, no
//! hover highlights or tooltips. A `UIHoverGestureRecognizer` on winit's view reports both; its
//! positions become egui `PointerMoved` events (UIKit points are egui points) and its end a
//! `PointerGone`. A `UIPencilInteraction` reports the double tap on the Pencil's side; when the
//! user's preferred action (Settings ▸ Apple Pencil) is to switch to the eraser or to the previous
//! tool, the app gets [`taken_tap`] and swaps the mask brush between Add and Erase.
//!
//! Drop the hover part once eframe moves to a winit that reports iOS hover (0.31 does).
#![allow(unsafe_code)] // Objective-C class definitions and the window handle's UIView pointer

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{ClassType, DefinedClass, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_foundation::NSObjectProtocol;
use objc2_ui_kit::{
    UIGestureRecognizer, UIGestureRecognizerState, UIHoverGestureRecognizer, UIPencilInteraction, UIPencilInteractionDelegate,
    UIPencilPreferredAction, UIView,
};

thread_local! {
    // main thread only: UIKit calls the handlers there, and the shell's frames run there
    static QUEUE: RefCell<Vec<egui::Event>> = const { RefCell::new(Vec::new()) };
    static TAPPED: Cell<bool> = const { Cell::new(false) };
    static HANDLER: RefCell<Option<Retained<Handler>>> = const { RefCell::new(None) };
    static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
}

/// The view hovering is reported in (winit's).
struct Ivars {
    view: Retained<UIView>,
}

define_class!(
    /// Target of the hover recognizer and delegate of the Pencil interaction.
    #[unsafe(super(NSObject))]
    #[name = "LCPencilHandler"]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct Handler;

    unsafe impl NSObjectProtocol for Handler {}

    impl Handler {
        #[unsafe(method(hover:))]
        fn hover(&self, recognizer: &UIGestureRecognizer) {
            let state = recognizer.state();
            let event = if state == UIGestureRecognizerState::Began || state == UIGestureRecognizerState::Changed {
                let p = recognizer.locationInView(Some(&self.ivars().view));
                egui::Event::PointerMoved(egui::pos2(p.x as f32, p.y as f32))
            } else {
                egui::Event::PointerGone
            };
            QUEUE.with_borrow_mut(|q| q.push(event));
            repaint();
        }
    }

    unsafe impl UIPencilInteractionDelegate for Handler {
        #[unsafe(method(pencilInteractionDidTap:))]
        fn pencil_interaction_did_tap(&self, _interaction: &UIPencilInteraction) {
            let action = UIPencilInteraction::preferredTapAction(self.mtm());
            if action == UIPencilPreferredAction::SwitchEraser || action == UIPencilPreferredAction::SwitchPrevious {
                TAPPED.set(true);
                repaint();
            }
        }
    }
);

fn repaint() {
    CONTEXT.with_borrow(|c| {
        if let Some(c) = c {
            c.request_repaint();
        }
    });
}

/// Adds the hover recognizer and the Pencil interaction to winit's view, once.
pub fn install(handle: raw_window_handle::RawWindowHandle, ctx: &egui::Context) {
    let raw_window_handle::RawWindowHandle::UiKit(h) = handle else { return };
    CONTEXT.with_borrow_mut(|c| *c = Some(ctx.clone()));
    if HANDLER.with_borrow(Option::is_some) {
        return;
    }
    // SAFETY: on iOS the window handle's `ui_view` is winit's UIView, alive while the window is
    let view: &UIView = unsafe { h.ui_view.cast::<UIView>().as_ref() };
    let mtm = view.mtm();
    let handler = Handler::alloc(mtm).set_ivars(Ivars { view: view.retain() });
    // SAFETY: NSObject's designated initializer
    let handler: Retained<Handler> = unsafe { msg_send![super(handler), init] };
    // SAFETY: `hover:` is a method of the target taking the recognizer; the target is kept alive
    // in HANDLER for the app's lifetime (recognizers don't retain their targets)
    let hover = unsafe { UIHoverGestureRecognizer::initWithTarget_action(UIHoverGestureRecognizer::alloc(mtm), Some(&handler), Some(sel!(hover:))) };
    view.addGestureRecognizer(&hover);
    let pencil = UIPencilInteraction::new(mtm);
    pencil.setDelegate(Some(ProtocolObject::from_ref(&*handler)));
    view.addInteraction(ProtocolObject::from_ref(&*pencil));
    HANDLER.with_borrow_mut(|h| *h = Some(handler));
    let _ = Handler::class();
}

/// Moves the hover positions reported since the last frame into `raw`. A touch in progress
/// takes the pointer itself (UIKit reports no hover then).
pub fn input(raw: &mut egui::RawInput) {
    QUEUE.with_borrow_mut(|q| raw.events.append(q));
}

/// Whether the Pencil was double-tapped (to switch to the eraser) since the last call.
pub fn taken_tap() -> bool {
    TAPPED.replace(false)
}
