use tracing::{error, info, warn};

pub trait PrintError<T, E> {
    fn ok_or_warn(self) -> Option<T>;
    fn ok_or_log(self) -> Option<T>;
    fn into_log(self) -> ();
}
impl<T: std::fmt::Debug, E: std::fmt::Display> PrintError<T, E> for Result<T, E> {
    fn ok_or_warn(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                warn!("{}", e);
                None
            }
        }
    }
    fn ok_or_log(self) -> Option<T> {
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
