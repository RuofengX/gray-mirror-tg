use tracing::{error, info, warn};

pub trait PrintError<T, E> {
    fn unwrap_or_warn(self) -> Option<T>;
    fn unwrap_or_log(self) -> Option<T>;
    fn into_log(self) -> ();
}
impl<T: std::fmt::Debug, E: std::fmt::Display> PrintError<T, E> for Result<T, E> {
    fn unwrap_or_warn(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                warn!("{}", e);
                None
            }
        }
    }
    fn unwrap_or_log(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                error!("{}", e);
                None
            }
        }
    }
    fn into_log(self) -> () {
        match self {
            Ok(t) => info!("{:?}", t),
            Err(e) => warn!("{}", e),
        }
    }
}
