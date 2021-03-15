use base64;
use sha1::Sha1;

pub(crate) fn convert_key(input: &[u8]) -> String {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::from(input);
    sha1.update(WS_GUID);
    base64::encode(&sha1.digest().bytes())
}
