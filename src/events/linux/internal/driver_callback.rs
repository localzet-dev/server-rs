use crate::events::linux::CallbackType;
#[derive(Clone, Debug)]
pub struct DriverCallback {
    pub id: String,
    pub callback_type: CallbackType,
    pub enabled: bool,
}
