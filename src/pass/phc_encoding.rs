/*
 * Copyright Rodolphe Breard (2016)
 * Author: Rodolphe Breard (2016)
 *
 * This software is a computer program whose purpose is to [describe
 * functionalities and technical features of your software].
 *
 * This software is governed by the CeCILL  license under French law and
 * abiding by the rules of distribution of free software.  You can  use,
 * modify and/ or redistribute the software under the terms of the CeCILL
 * license as circulated by CEA, CNRS and INRIA at the following URL
 * "http://www.cecill.info".
 *
 * As a counterpart to the access to the source code and  rights to copy,
 * modify and redistribute granted by the license, users are provided only
 * with a limited warranty  and the software's author,  the holder of the
 * economic rights,  and the successive licensors  have only  limited
 * liability.
 *
 * In this respect, the user's attention is drawn to the risks associated
 * with loading,  using,  modifying and/or developing or reproducing the
 * software by the user in light of its specific status of free software,
 * that may mean  that it is complicated to manipulate,  and  that  also
 * therefore means  that it is reserved for developers  and  experienced
 * professionals having in-depth computer knowledge. Users are therefore
 * encouraged to load and test the software's suitability as regards their
 * requirements in conditions enabling the security of their systems and/or
 * data to be ensured and,  more generally, to use and operate it in the
 * same conditions as regards security.
 *
 * The fact that you are presently reading this means that you have had
 * knowledge of the CeCILL license and that you accept its terms.
 */



use super::{PASSWORD_MIN_LEN, PASSWORD_MAX_LEN};
use super::{ErrorCode, generate_salt};
use std::collections::HashMap;
use rustc_serialize::hex::FromHex;
use rustc_serialize::hex::ToHex;

use ring;


#[repr(C)]
#[derive(Clone, Copy)]
pub enum HashFunction {
    Sha1 = 1,
    Sha256 = 2,
    Sha512 = 3,
}

macro_rules! get_salt {
    ($salt:expr) => {{
        match $salt.to_owned() {
            Some(s) => s,
            None => generate_salt(8),
        }
    }}
}

macro_rules! get_param {
    ($h:expr, $k:expr, $t:ty, $default:expr) => {{
        if $h.contains_key($k) {
            $h.get($k).unwrap().parse::<$t>().unwrap_or($default)
        } else {
            $default
        }
    }}
}

use std::fmt;

/// We define the format: $<id>[$<param>=<value>(,<param>=<value>)*][$<salt>[$<hash>]]
pub struct PHCEncoded {
    pub id: Option<String>,
    pub parameters: HashMap<String, String>,
    // we need to keep track of what order the parameters were parsed in
    // so that serializing the produces the same output as the input
    parameters_order: Vec<(String, String)>,
    pub salt: Option<String>,
    pub hash: Option<String>,
}

impl fmt::Display for PHCEncoded {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let empty_string: String = "".to_string();

        let scheme_id = match self.id {
            Some(ref scheme) => scheme,
            None => &empty_string,
        };

        write!(f, "${}", scheme_id);
        if self.parameters_order.len() > 0 {
            write!(f, "$");

            let mut parameters = self.parameters_order.iter();

            if let Some(&(ref param, ref value)) = parameters.next() {
                write!(f, "{}={}", param, value);

                while let Some(&(ref p, ref v)) = parameters.next() {
                    write!(f, ",{}={}", p, v);
                }
            }
        }
        if let Some(ref s) = self.salt {
            // TODO: remember to base64 encode the salt
            write!(f, "${}", s);
        }
        write!(f, "")
    }
}

impl PHCEncoded {
    pub fn from_string(pch_formatted: &str) -> Result<PHCEncoded, ErrorCode> {
        let mut encoded = PHCEncoded {
            id: None,
            parameters: HashMap::new(),
            parameters_order: Vec::new(),
            salt: None,
            hash: None,
        };

        let mut parts: Vec<&str> = pch_formatted.split("$").collect();
        if parts.len() < 2 || parts.len() > 5 {
            return Err(ErrorCode::InvalidPasswordFormat);
        }

        parts.reverse();
        parts.pop(); // Remove the first empty element.

        encoded.id = Some(parts.pop().unwrap().to_string());

        let mut segment: &str = match parts.pop() {
            Some(some) => some,
            None => return Ok(encoded), // Format: $id
        };

        // Now look for an optional list of parameter value pairs
        // aka: [$<param>=<value>(,<param>=<value>)*]
        if segment.contains("=") {
            let params: Vec<&str> = segment.split(",").collect();
            for item in params.iter() {
                // the value for each parameter may only contain the characters [a-zA-Z0-9/+.-]
                // (lowercase letters, uppercase letters, digits, /, +, . and -)
                let pair: Vec<&str> = item.split('=').collect();
                if pair.len() == 2 {
                    let param: String = pair[0].to_string();
                    let value: String = pair[1].to_string();
                    encoded.parameters.insert(param.clone(), value.clone());
                    encoded.parameters_order.push((param, value))
                } else {
                    return Err(ErrorCode::InvalidPasswordFormat);
                }
            }
            segment = match parts.pop() {
                Some(some) => some,
                None => return Ok(encoded),
            };
        } else if segment.len() == 0 {
            // The parameters section may also be empty
            segment = match parts.pop() {
                Some(some) => some,
                None => return Ok(encoded),
            };
        }

        if segment == "" {
            return Err(ErrorCode::InvalidPasswordFormat);
        } else {
            // TODO: use the correct encoding of the salt based on the $id
            encoded.salt = Some(segment.to_string())
        }

        // even if the salt is provided the hash is optional
        encoded.hash = match parts.pop() {
            Some(some) => Some(some.to_string()),
            None => return Ok(encoded),
        };

        return Ok(encoded);
    }
}
