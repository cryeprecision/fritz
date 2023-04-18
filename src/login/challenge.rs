use sha2::Sha256;
use std::{num::ParseIntError, str::FromStr};
use thiserror::Error;

#[derive(Debug, Default)]
pub struct Pbkdf2Params {
    pub iterations: u32,
    pub salt: [u8; 16],
}

impl Pbkdf2Params {
    pub fn hash(&self, password: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        pbkdf2::pbkdf2_hmac::<Sha256>(password, &self.salt, self.iterations, &mut out);
        out
    }
}

#[derive(Debug)]
pub struct Challenge {
    pub statick: Pbkdf2Params,
    pub dynamic: Pbkdf2Params,
}

#[derive(Error, Debug)]
pub enum ChallengeParseError {
    #[error("invalid format")]
    Format,
    #[error("unsupported version")]
    Version,
    #[error("invalid salt: {0}")]
    Salt(#[from] hex::FromHexError),
    #[error("couldn't parse iterations: {0}")]
    Iterations(#[from] ParseIntError),
}

impl FromStr for Challenge {
    type Err = ChallengeParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splits = s.split('$').collect::<Vec<_>>();
        if splits.len() != 5 {
            return Err(ChallengeParseError::Format);
        }

        let version = splits[0];
        let static_iter = splits[1];
        let static_salt = splits[2];
        let dynamic_iter = splits[3];
        let dynamic_salt = splits[4];

        if version != "2" {
            return Err(ChallengeParseError::Version);
        }

        let mut static_salt_buf = [0u8; 16];
        hex::decode_to_slice(static_salt, &mut static_salt_buf)?;

        let mut dynamic_salt_buf = [0u8; 16];
        hex::decode_to_slice(dynamic_salt, &mut dynamic_salt_buf)?;

        Ok(Challenge {
            statick: Pbkdf2Params {
                iterations: static_iter.parse()?,
                salt: static_salt_buf,
            },
            dynamic: Pbkdf2Params {
                iterations: dynamic_iter.parse()?,
                salt: dynamic_salt_buf,
            },
        })
    }
}

impl Challenge {
    pub fn hash(&self, password: &[u8]) -> String {
        let static_hash = self.statick.hash(password);
        let dynamic_hash = self.dynamic.hash(&static_hash);
        let dynamic_salt_hex = hex::encode(self.dynamic.salt);
        let dynamic_hash_hex = hex::encode(dynamic_hash);
        format!("{dynamic_salt_hex}${dynamic_hash_hex}")
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::Challenge;

    #[test]
    fn parse() {
        const CHALLENGE: &str = "\
            2$60000$d4949767019d1e6eed27c27f404c7aa7$\
            6000$4f3415a3b5396a9675d08906ee6a6933";
        const RESPONSE: &str = "\
            4f3415a3b5396a9675d08906ee6a6933$\
            16a4a11987d802c6f3e67d91d1425b5a0eade78561a5810ef905372ab1da53ca";

        let ch = Challenge::from_str(CHALLENGE).unwrap();

        assert_eq!(ch.statick.iterations, 60000);
        assert_eq!(ch.dynamic.iterations, 6000);

        assert_eq!(
            ch.statick.salt,
            [212, 148, 151, 103, 1, 157, 30, 110, 237, 39, 194, 127, 64, 76, 122, 167]
        );
        assert_eq!(
            ch.dynamic.salt,
            [79, 52, 21, 163, 181, 57, 106, 150, 117, 208, 137, 6, 238, 106, 105, 51]
        );

        let first_hash = ch.statick.hash(b"vorab9049");
        assert_eq!(
            first_hash,
            [
                173, 73, 26, 0, 69, 3, 2, 226, 26, 14, 168, 166, 149, 148, 120, 114, 4, 167, 182,
                35, 234, 201, 114, 174, 21, 114, 197, 66, 252, 236, 254, 29
            ]
        );

        let second_hash = ch.dynamic.hash(&first_hash);
        assert_eq!(
            second_hash,
            [
                22, 164, 161, 25, 135, 216, 2, 198, 243, 230, 125, 145, 209, 66, 91, 90, 14, 173,
                231, 133, 97, 165, 129, 14, 249, 5, 55, 42, 177, 218, 83, 202
            ]
        );

        let response = ch.hash(b"vorab9049");
        assert_eq!(response, RESPONSE);
    }

    #[test]
    fn get_response() {
        const CHALLENGE: &str =
            "2$60000$d4949767019d1e6eed27c27f404c7aa7$6000$662dc618ec19bc5012b272f53b805c01";
        const PASSWORD: &[u8] = b"vorab9049";

        println!(
            "{:#?}",
            Challenge::from_str(CHALLENGE).unwrap().hash(PASSWORD)
        );
    }
}
