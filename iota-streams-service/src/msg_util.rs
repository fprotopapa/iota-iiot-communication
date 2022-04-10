/// Enum MsgType maps u32 to message types defined by rpc calls
#[derive(Debug)]
#[repr(u32)]
#[allow(dead_code)]
pub enum MsgType {
    Error = 0,
    CreateNewAuthor,
    CreateNewSubscriber,
    AddSubscriber,
    ReceiveKeyload,
    SendMessage,
    ReceiveMessages,
    Unknown,
}
/// Convert u32 to MsgType
#[allow(dead_code)]
pub fn convert_to_msgtype(num: u32) -> MsgType {
    match num {
        0 => MsgType::Error,
        1 => MsgType::CreateNewAuthor,
        2 => MsgType::CreateNewSubscriber,
        3 => MsgType::AddSubscriber,
        4 => MsgType::ReceiveKeyload,
        5 => MsgType::SendMessage,
        6 => MsgType::ReceiveMessages,
        _ => MsgType::Unknown,
    }
}
/// Convert MsgType to u32
#[allow(dead_code)]
pub fn convert_from_msgtype(msg_type: MsgType) -> u32 {
    match msg_type {
        MsgType::Error => 0,
        MsgType::CreateNewAuthor => 1,
        MsgType::CreateNewSubscriber => 2,
        MsgType::AddSubscriber => 3,
        MsgType::ReceiveKeyload => 4,
        MsgType::SendMessage => 5,
        MsgType::ReceiveMessages => 6,
        MsgType::Unknown => 7,
    }
}
