use color_eyre::eyre::{Result, eyre};
use std::sync::{MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard};

/// Extension trait for handling results of mutex and rwlock guards, converting poison errors to eyre errors.
pub(crate) trait GuardResultExt<'a, T> {
    type Guard;
    fn eyre(self) -> Result<Self::Guard>;
}

impl<'a, T> GuardResultExt<'a, T> for Result<MutexGuard<'a, T>, PoisonError<MutexGuard<'a, T>>> {
    type Guard = MutexGuard<'a, T>;
    fn eyre(self) -> Result<Self::Guard> {
        self.map_err(|e| eyre!("Mutex poisoned: {e}"))
    }
}

impl<'a, T> GuardResultExt<'a, T>
    for Result<RwLockReadGuard<'a, T>, PoisonError<RwLockReadGuard<'a, T>>>
{
    type Guard = RwLockReadGuard<'a, T>;
    fn eyre(self) -> Result<Self::Guard> {
        self.map_err(|e| eyre!("RwLock (read) poisoned: {e}"))
    }
}

impl<'a, T> GuardResultExt<'a, T>
    for Result<RwLockWriteGuard<'a, T>, PoisonError<RwLockWriteGuard<'a, T>>>
{
    type Guard = RwLockWriteGuard<'a, T>;
    fn eyre(self) -> Result<Self::Guard> {
        self.map_err(|e| eyre!("RwLock (write) poisoned: {e}"))
    }
}
