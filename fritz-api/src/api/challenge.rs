//! Parse the challenge and generate a response for the challenge-response
//! login scheme used by the FRITZ!Box.
//!
//! <https://avm.de/fileadmin/user_upload/Global/Service/Schnittstellen/AVM_Technical_Note_-_Session_ID_deutsch_2021-05-03.pdf>

use std::borrow::Cow;
use std::str::FromStr;

use anyhow::Context;
use pbkdf2::pbkdf2_hmac;
use serde::Deserialize;
use sha2::Sha256;

#[derive(Debug, Clone)]
pub struct Challenge {
    pub salt_1: [u8; 16],
    pub rounds_1: u32,
    pub salt_2: [u8; 16],
    pub rounds_2: u32,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub salt: [u8; 16],
    pub hash: [u8; 32],
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let salt = hex::encode(self.salt);
        let hash = hex::encode(self.hash);
        write!(f, "{salt}${hash}")
    }
}

impl FromStr for Challenge {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Challenge> {
        fn get_splits(s: &str) -> anyhow::Result<[&str; 5]> {
            let mut iter = s.split('$');
            Ok([
                iter.next().context("get version part")?,
                iter.next().context("get rounds_1 part")?,
                iter.next().context("get salt_1 part")?,
                iter.next().context("get rounds_2 part")?,
                iter.next().context("get salt_2 part")?,
            ])
        }

        let [version, rounds_1, salt_1, rounds_2, salt_2] = get_splits(s)?;

        if version != "2" {
            return Err(anyhow::anyhow!("invalid version"));
        }

        let rounds_1 = rounds_1.parse().context("parse rounds_1")?;
        let rounds_2 = rounds_2.parse().context("parse rounds_2")?;

        let mut salt_1_buf = [0u8; 16];
        hex::decode_to_slice(salt_1, &mut salt_1_buf)?;

        let mut salt_2_buf = [0u8; 16];
        hex::decode_to_slice(salt_2, &mut salt_2_buf)?;

        Ok(Challenge {
            salt_1: salt_1_buf,
            rounds_1,
            salt_2: salt_2_buf,
            rounds_2,
        })
    }
}
impl<'de> Deserialize<'de> for Challenge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let challenge = Cow::<'de, str>::deserialize(deserializer)?;
        challenge.parse().map_err(serde::de::Error::custom)
    }
}

impl Challenge {
    pub fn make_response(&self, password: &str) -> Response {
        let mut hash_1_buf = [0u8; 32];
        pbkdf2_hmac::<Sha256>(
            password.as_bytes(),
            &self.salt_1,
            self.rounds_1,
            &mut hash_1_buf,
        );

        let mut hash_2_buf = [0u8; 32];
        pbkdf2_hmac::<Sha256>(
            hash_1_buf.as_slice(),
            &self.salt_2,
            self.rounds_2,
            &mut hash_2_buf,
        );

        Response {
            salt: self.salt_2,
            hash: hash_2_buf,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::Challenge;

    #[test]
    fn parse() {
        const CHALLENGE: &str =
            "2$60000$d4949767019d1e6eed27c27f404c7aa7$6000$4f3415a3b5396a9675d08906ee6a6933";
        const RESPONSE: &str = "4f3415a3b5396a9675d08906ee6a6933$16a4a11987d802c6f3e67d91d1425b5a0eade78561a5810ef905372ab1da53ca";

        let ch = Challenge::from_str(CHALLENGE).unwrap();

        assert_eq!(ch.rounds_1, 60000);
        assert_eq!(ch.rounds_2, 6000);

        assert_eq!(
            ch.salt_1,
            [212, 148, 151, 103, 1, 157, 30, 110, 237, 39, 194, 127, 64, 76, 122, 167]
        );
        assert_eq!(
            ch.salt_2,
            [79, 52, 21, 163, 181, 57, 106, 150, 117, 208, 137, 6, 238, 106, 105, 51]
        );

        let response = ch.make_response("vorab9049");
        assert_eq!(response.to_string(), RESPONSE);
    }
}
