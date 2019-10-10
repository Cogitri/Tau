use gdk::enums::key;
use glib::object::{ObjectExt, WeakRef};
use glib::SignalHandlerId;
use gtk::prelude::*;
use gtk::{Inhibit, Notebook, Widget};
use log::debug;
use std::cell::RefCell;
use std::rc::Rc;

/// Records history of visited tabs
pub struct ViewHistory {
    /// weak reference to notebook
    parent: WeakRef<Notebook>,
    /// list of visited page widgets
    book: Vec<WeakRef<Widget>>,
    /// current index of the history
    index: usize,
    /// callback handler for tab switching
    switch_handler: Option<SignalHandlerId>,
    /// callback handler to stop cycling
    stop_handler: Option<SignalHandlerId>,
    /// callback handler for adding child
    add_handler: Option<SignalHandlerId>,
    /// callback handler for removing child
    remove_handler: Option<SignalHandlerId>,
}

/// Extension trait for `ViewHistory`
pub trait ViewHistoryExt {
    /// Update history when not cycling through it
    /// # Panics
    /// Panics if `self` is already borrowed
    fn update(&self, child: &Widget);
    /// Set up event handlers when creating the history
    /// # Panics
    /// Panics if `self` is already borrowed
    fn connect_events(&self);
    /// Prepare for history being recorded
    /// # Panics
    /// Panics if `self` is already borrowed
    fn listen(&self);
    /// Consolidate history after cycling finished
    /// # Panics
    /// Panics if `self` is already borrowed
    fn write_history(&self);
    /// Cycle to previous tab in history
    /// # Panics
    /// Panics if `self` is already borrowed
    fn cycle_backward(&self);
    /// Cycle to next tab in history
    /// # Panics
    /// Panics if `self` is already borrowed
    fn cycle_forward(&self);
    /// Cycle towards the position of `index`
    /// # Panics
    /// Panics if `self` is already borrowed
    fn cycle_to_index(&self, index: i32);
}

impl ViewHistory {
    /// Create new history
    pub fn new(notebook: &Notebook) -> Rc<RefCell<Self>> {
        let book = notebook
            .get_children()
            .drain(..)
            .map(|w| w.downgrade())
            .collect();

        let history = Rc::new(RefCell::new(Self {
            book,
            parent: notebook.downgrade(),
            index: 0,
            switch_handler: None,
            stop_handler: None,
            add_handler: None,
            remove_handler: None,
        }));

        history.connect_events();
        history
    }
}

impl ViewHistoryExt for Rc<RefCell<ViewHistory>> {
    fn connect_events(&self) {
        {
            let mut view_history = self.borrow_mut();
            if let Some(notebook) = view_history.parent.upgrade() {
                if view_history.add_handler.is_none() {
                    view_history.add_handler = Some(notebook.connect_page_added(
                        enclose!((self => vh) move |_, w, _| {
                            vh.borrow_mut().book.push((*w).downgrade());
                        }),
                    ));
                }
                if view_history.remove_handler.is_none() {
                    notebook.connect_page_removed(enclose!((self => vh) move |_, w, _| {
                        vh.borrow_mut().book.retain(|x| {
                            if let Some(listed_widget) = x.upgrade() {
                                listed_widget != *w
                            } else {
                                false
                            }
                        });
                    }));
                }
            }
        }
        self.listen();
    }

    fn listen(&self) {
        let mut view_history = self.borrow_mut();
        if let Some(notebook) = view_history.parent.upgrade() {
            // unregister stop handler
            if let Some(handler) = view_history.stop_handler.take() {
                notebook.disconnect(handler);
                view_history.stop_handler = None;
            }

            // register switch handler
            view_history.switch_handler = Some(notebook.connect_switch_page(
                enclose!((self => vh) move |_, w, _| {
                    vh.update(w);
                }),
            ));
        }
    }

    fn update(&self, child: &Widget) {
        let mut view_history = self.borrow_mut();

        // check if any tabs are listed
        if let Some(first_ref) = view_history.book.first() {
            if let Some(first_widget) = first_ref.upgrade() {
                if first_widget != *child {
                    view_history.book = {
                        let mut new_history =
                            vec![child.clone().downgrade(), first_widget.downgrade()];
                        new_history.extend(view_history.book.drain(1..).filter(|weak_ref| {
                            if let Some(w) = weak_ref.upgrade() {
                                w != *child
                            } else {
                                false
                            }
                        }));
                        new_history
                    }
                }
            } else {
                view_history.book = {
                    let mut new_history = vec![child.downgrade()];
                    new_history.extend(view_history.book.drain(1..).filter(|weak_ref| {
                        if let Some(w) = weak_ref.upgrade() {
                            w != *child
                        } else {
                            false
                        }
                    }));
                    new_history
                }
            }
        } else {
            view_history.book.push(child.downgrade());
        }
    }

    fn write_history(&self) {
        let (len, index) = {
            let view_history = self.borrow();
            (view_history.book.len(), view_history.index)
        };

        if len > 0 && index != 0 {
            debug!("Write tab history");
            {
                let mut view_history = self.borrow_mut();
                view_history.book.swap(0, index);
                view_history.index = 0;
            }
            self.listen();
        }
    }

    fn cycle_to_index(&self, index: i32) {
        let mut view_history = self.borrow_mut();
        let len = view_history.book.len() as i32;

        if len > 0 {
            if let Some(notebook) = view_history.parent.upgrade() {
                // make sure index is in bounds
                let mut checked_index = index % len;
                if checked_index < 0 {
                    checked_index += len;
                }

                view_history.index = checked_index as usize;

                // remove switch handler if necessary
                if let Some(handler_id) = view_history.switch_handler.take() {
                    notebook.disconnect(handler_id);
                    view_history.switch_handler = None;
                }

                // register stop handler
                if view_history.stop_handler == None {
                    view_history.stop_handler = Some(notebook.connect_key_release_event(
                        enclose!((self => vh) move |_, ek| {
                            match ek.get_keyval() {
                                key::Control_L | key::Control_R => {
                                    vh.write_history();
                                }
                                _ => {}
                            }
                            Inhibit(false)
                        }),
                    ));
                }

                // cycle to indexed tab
                if let Some(notebook) = view_history.parent.upgrade() {
                    if let Some(Some(w)) = view_history
                        .book
                        .get(view_history.index)
                        .map(|wr| wr.upgrade())
                    {
                        if let Some(page_num) = notebook.page_num(&w) {
                            notebook.set_property_page(page_num as i32);
                        }
                    }
                }
            }
        }
    }

    fn cycle_backward(&self) {
        debug!("Cycle tab history backward");
        let index = { self.borrow().index as i32 + 1 };
        self.cycle_to_index(index);
    }

    fn cycle_forward(&self) {
        debug!("Cycle tab history forward");
        let index = { self.borrow().index as i32 - 1 };
        self.cycle_to_index(index);
    }
}

impl Drop for ViewHistory {
    /// unregister all active handlers when dropping
    fn drop(&mut self) {
        if let Some(notebook) = self.parent.upgrade() {
            if let Some(switch_handler) = self.switch_handler.take() {
                notebook.disconnect(switch_handler);
            }
            if let Some(stop_handler) = self.stop_handler.take() {
                notebook.disconnect(stop_handler);
            }
            if let Some(add_handler) = self.add_handler.take() {
                notebook.disconnect(add_handler);
            }
            if let Some(remove_handler) = self.remove_handler.take() {
                notebook.disconnect(remove_handler);
            }
        }
    }
}
