pub struct CleanupTask<F>
where
    F: Fn() + Send + 'static,
{
    callback: F,
}

impl<F> CleanupTask<F>
where
    F: Fn() + Send + 'static,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> Drop for CleanupTask<F>
where
    F: Fn() + Send + 'static,
{
    fn drop(&mut self) {
        (self.callback)();
    }
}
