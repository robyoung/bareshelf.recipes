use crate::error::Error;

use ring::hmac;

pub(crate) fn encode_share_token(secret: &[u8], uid: u32) -> Result<String, Error> {
    let message =
        serde_json::to_vec(&uid).map_err(|_| Error::Other("cannot create token".to_string()))?;

    let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA384, secret), &message);
    let token = format!(
        "{}.{}",
        base64::encode_config(&message, base64::URL_SAFE_NO_PAD),
        base64::encode_config(&tag.as_ref(), base64::URL_SAFE_NO_PAD)
    );

    Ok(token)
}

#[inline]
fn invalid_token<T>(_: T) -> Error {
    Error::Other("invalid token".to_string())
}

struct Nothing;

pub(crate) fn decode_share_token(secret: &[u8], token: &str) -> Result<u32, Error> {
    let parts: Vec<_> = token.split('.').collect();
    if parts.len() != 2 {
        return Err(invalid_token(Nothing));
    }
    let (message, tag) = (
        base64::decode_config(parts[0], base64::URL_SAFE_NO_PAD).map_err(invalid_token)?,
        base64::decode_config(parts[1], base64::URL_SAFE_NO_PAD).map_err(invalid_token)?,
    );
    hmac::verify(&hmac::Key::new(hmac::HMAC_SHA384, secret), &message, &tag)
        .map_err(invalid_token)?;
    let uid: u32 = serde_json::from_slice(&message).map_err(invalid_token)?;

    Ok(uid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_and_decode_a_token() {
        let secret = "a secret".as_bytes();
        let uid = 1234;

        let token = encode_share_token(&secret, uid).unwrap();
        let decoded_uid = decode_share_token(&secret, &token).unwrap();

        assert_eq!(uid, decoded_uid);
    }

    #[test]
    fn fail_decoding_a_token() {
        let secret = "a secret".as_bytes();
        let bad_token = "a bad token";

        let result = decode_share_token(&secret, &bad_token);
        assert!(result.is_err());
        assert_eq!(format!("{}", invalid_token(Nothing)), format!("{}", result.err().unwrap()));
    }
}
