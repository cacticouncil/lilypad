use std::{
    cell::{Cell, RefCell, RefMut},
    ops::Range,
    sync::{Arc, Weak},
};

use druid::{
    piet::HitTestPoint,
    text::{ImeHandlerRef, InputHandler, Selection, TextAction},
    Point, Rect,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ImeLock {
    None,
    ReadWrite,
    Read,
}

/* -------------------------------- Component ------------------------------- */

pub struct ImeComponent {
    ime_session: Arc<RefCell<ImeSession>>,
    lock: Arc<Cell<ImeLock>>,
}

impl Default for ImeComponent {
    fn default() -> Self {
        let session = ImeSession {
            orgin: Point::ZERO,
            external_text_change: None,
            external_action: None,
        };
        ImeComponent {
            ime_session: Arc::new(RefCell::new(session)),
            lock: Arc::new(Cell::new(ImeLock::None)),
        }
    }
}

impl ImeComponent {
    pub fn ime_handler(&self) -> impl ImeHandlerRef {
        ImeSessionRef {
            inner: Arc::downgrade(&self.ime_session),
            lock: self.lock.clone(),
        }
    }

    pub fn borrow_mut(&self) -> RefMut<'_, ImeSession> {
        self.ime_session.borrow_mut()
    }
}

/* ------------------------------- Session Ref ------------------------------ */

struct ImeSessionRef {
    inner: Weak<RefCell<ImeSession>>,
    lock: Arc<Cell<ImeLock>>,
}

impl ImeHandlerRef for ImeSessionRef {
    fn is_alive(&self) -> bool {
        Weak::strong_count(&self.inner) > 0
    }

    fn acquire(&self, mutable: bool) -> Option<Box<dyn druid::text::InputHandler + 'static>> {
        let lock = if mutable {
            ImeLock::ReadWrite
        } else {
            ImeLock::Read
        };
        self.lock.replace(lock);
        Weak::upgrade(&self.inner)
            .map(ImeSessionHandle::new)
            .map(|doc| Box::new(doc) as Box<dyn InputHandler>)
    }

    fn release(&self) -> bool {
        self.lock.replace(ImeLock::None) == ImeLock::ReadWrite
    }
}

/* --------------------------------- Session -------------------------------- */

pub struct ImeSession {
    orgin: Point,
    external_text_change: Option<String>,
    external_action: Option<TextAction>,
}

impl ImeSession {
    pub fn take_external_text_change(&mut self) -> Option<String> {
        self.external_text_change.take()
    }

    pub fn take_external_action(&mut self) -> Option<TextAction> {
        self.external_action.take()
    }
}

/* --------------------------------- Handle --------------------------------- */

struct ImeSessionHandle {
    inner: Arc<RefCell<ImeSession>>,
}

impl ImeSessionHandle {
    fn new(inner: Arc<RefCell<ImeSession>>) -> Self {
        ImeSessionHandle { inner }
    }
}

impl InputHandler for ImeSessionHandle {
    fn selection(&self) -> Selection {
        Selection::default()
    }

    fn set_selection(&mut self, _selection: Selection) {}

    fn composition_range(&self) -> Option<std::ops::Range<usize>> {
        None
    }

    fn set_composition_range(&mut self, _range: Option<std::ops::Range<usize>>) {}

    fn is_char_boundary(&self, _i: usize) -> bool {
        false
    }

    fn len(&self) -> usize {
        0
    }

    fn slice(&self, _range: std::ops::Range<usize>) -> std::borrow::Cow<str> {
        std::borrow::Cow::Borrowed("")
    }

    fn replace_range(&mut self, _range: Range<usize>, text: &str) {
        self.inner.borrow_mut().external_text_change = Some(text.to_string());
    }

    fn hit_test_point(&self, _point: Point) -> HitTestPoint {
        HitTestPoint::default()
    }

    fn line_range(
        &self,
        _index: usize,
        _affinity: druid::text::Affinity,
    ) -> std::ops::Range<usize> {
        0..0
    }

    fn bounding_box(&self) -> Option<Rect> {
        None
    }

    fn slice_bounding_box(&self, _range: std::ops::Range<usize>) -> Option<Rect> {
        Some(Rect::ZERO.with_origin(self.inner.borrow().orgin))
    }

    fn handle_action(&mut self, action: druid::text::TextAction) {
        self.inner.borrow_mut().external_action = Some(action);
    }
}
