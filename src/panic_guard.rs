use std::sync::{Arc, Mutex};

type PanicHookType = (dyn for<'r, 's> Fn(&'r std::panic::PanicInfo<'s>) + Send + Sync + 'static);

/// Custom scopeguard-like struct that wraps a panic hook function and a callback ("cleanup")
/// function, and in the case of a panic, calls the callback *before* the wrapped panic hook (i.e.
/// before printing the error message to stderr). We need this to e.g. switch back to the
/// non-alternate screen before printing the error message, so that it doesn't disappear into the
/// alternate screen.
pub struct GuardWithHook<F>
where
    F: FnOnce() + Sync + Send + 'static,
{
    original_hook: Arc<PanicHookType>,
    // Use Option to deonte whether the function has been called or not.
    callback: Arc<Mutex<Option<F>>>,
}

impl<F> GuardWithHook<F>
where
    F: FnOnce() + Sync + Send + 'static,
{
    /// Store a callback function and the current panic hook, and install a new panic hook that
    /// first calls the callback, and then the original panic hook.
    pub fn new(callback: F) -> Self {
        let callback = Arc::new(Mutex::new(Some(callback)));
        let callback_copy = callback.clone();

        let original_hook: Arc<PanicHookType> = Arc::from(std::panic::take_hook());
        let original_hook_copy = original_hook.clone();

        // TODO: use std::panic::update_hook once it is stabilized...
        // see: https://doc.rust-lang.org/std/panic/fn.update_hook.html
        // and: https://github.com/rust-lang/rust/issues/92649
        std::panic::set_hook(Box::new(move |info| {
            if let Ok(mut callback) = callback_copy.try_lock() {
                if let Some(callback) = callback.take() {
                    callback();
                }
            }
            (*original_hook_copy)(info);
        }));

        Self {
            original_hook,
            callback,
        }
    }
}

impl<F> Drop for GuardWithHook<F>
where
    F: FnOnce() + Sync + Send + 'static,
{
    /// Restore the original panic hook, and call the callback.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            // Set the panic hook back to what it was before. Note that this can be done only if
            // we're not panicking (the hook can't be modified during a panic, of course).
            let original_hook = self.original_hook.clone();
            std::panic::set_hook(Box::new(move |info| (*original_hook)(info)));
        }

        // If callback has not been called yet (i.e. it is Some), call it
        if let Ok(mut callback) = self.callback.try_lock() {
            if let Some(callback) = callback.take() {
                callback();
            }
        }
    }
}