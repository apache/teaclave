// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! This module implements ECDSA (NIST P-256 curve) keys related functions. You
//! can export private key to a DER format or create a certificate with
//! extension for TLS-based remote attestation.

use crate::date::{seconds_to_datetime, DateTime};
use anyhow::{bail, Result};
use basic_utils::get_system_time_seconds;
use crypto::EcdsaKeyPair;
use yasna::models::UTCTime;

/// create_cert_with_extension makes a self-signed x509-v3 cert with SGX
/// attestation report as extensions.
/// @reference [Internet X.509 Public Key Infrastructure Certificate and
/// Certificate Revocation List (CRL) Profile][1]
///
/// [1]: https://tools.ietf.org/pdf/rfc5280.pdf
pub fn sign_csr_with_extension(
    key_pair: &EcdsaKeyPair,
    pub_key: &[u8],
    issuer: &str,
    subject: &str,
    alter_dns_names: [&str; 2],
    payload: &[u8],
    cert_valid_seconds: i64,
) -> Result<Vec<u8>> {
    use crate::cert::*;
    use bit_vec::BitVec;
    use yasna::models::ObjectIdentifier;

    // Construct useful OIDs.
    let ecdsa_with_sha256_oid = ObjectIdentifier::from_slice(&[1, 2, 840, 10045, 4, 3, 2]);
    let common_name_oid = ObjectIdentifier::from_slice(&[2, 5, 4, 3]);
    let ec_public_key_oid = ObjectIdentifier::from_slice(&[1, 2, 840, 10045, 2, 1]);
    let prime256v1_oid = ObjectIdentifier::from_slice(&[1, 2, 840, 10045, 3, 1, 7]);
    let comment_oid = ObjectIdentifier::from_slice(&[2, 16, 840, 1, 113_730, 1, 13]);
    let subject_alt_name_oid = ObjectIdentifier::from_slice(&[2, 5, 29, 17]);

    let alter_names = (
        DnsName::new(alter_dns_names[0].to_string()),
        DnsName::new(alter_dns_names[1].to_string()),
    );

    let (issue_ts, expire_ts) = valid_time_range(cert_valid_seconds)?;
    // Construct certificate with payload in extension in DER.
    let tbs_cert_der = yasna::construct_der(|writer| {
        let version = 2i8;
        let serial = 1u8;
        let cert_sign_algo = asn1_seq!(ecdsa_with_sha256_oid.clone());
        let issuer = asn1_seq!(asn1_seq!(asn1_seq!(
            common_name_oid.clone(),
            issuer.to_owned()
        )));
        let valid_range = asn1_seq!(issue_ts, expire_ts);
        let subject = asn1_seq!(asn1_seq!(asn1_seq!(
            common_name_oid.clone(),
            subject.to_string(),
        )));
        let pub_key = asn1_seq!(
            asn1_seq!(ec_public_key_oid, prime256v1_oid,),
            BitVec::from_bytes(pub_key),
        );

        let sgx_ra_cert_ext = asn1_seq!(comment_oid, payload.to_owned());

        let alter_names_payload = AlterNamesPayload::new(alter_names.0, alter_names.1);
        let alter_names = asn1_seq!(subject_alt_name_oid, alter_names_payload);

        let cert_ext = asn1_seq!(sgx_ra_cert_ext, alter_names);

        let tbs_cert = asn1_seq!(
            version,
            serial,
            cert_sign_algo,
            issuer,
            valid_range,
            subject,
            pub_key,
            cert_ext,
        );
        TbsCert::dump(writer, tbs_cert);
    });

    let sig = key_pair.sign(tbs_cert_der.as_slice()).unwrap();
    let sig_der = yasna::construct_der(|writer| writer.write_der(sig.as_ref()));

    let der = yasna::construct_der(|writer| {
        writer.write_sequence(|writer| {
            writer.next().write_der(tbs_cert_der.as_slice());
            CertSignAlgo::dump(writer.next(), asn1_seq!(ecdsa_with_sha256_oid.clone()));
            writer
                .next()
                .write_bitvec(&BitVec::from_bytes(sig_der.as_slice()));
        });
    });

    Ok(der)
}

pub fn sign_intermediate(
    key_pair: &EcdsaKeyPair,
    pub_key: &[u8],
    issuer: &str,
    subject: &str,
    cert_valid_seconds: i64,
) -> Result<Vec<u8>> {
    use crate::cert::*;
    use bit_vec::BitVec;
    use yasna::models::ObjectIdentifier;
    // Construct useful OIDs.
    let ecdsa_with_sha384_oid = ObjectIdentifier::from_slice(&[1, 2, 840, 10045, 4, 3, 3]);
    let common_name_oid = ObjectIdentifier::from_slice(&[2, 5, 4, 3]);
    let ec_public_key_oid = ObjectIdentifier::from_slice(&[1, 2, 840, 10045, 2, 1]);
    let prime256v1_oid = ObjectIdentifier::from_slice(&[1, 2, 840, 10045, 3, 1, 7]);
    let basic_constrain_oid = ObjectIdentifier::from_slice(&[2, 5, 29, 19]);

    let (issue_ts, expire_ts) = valid_time_range(cert_valid_seconds)?;
    // Construct certificate with payload in extension in DER.
    let tbs_cert_der = yasna::construct_der(|writer| {
        let version = 2i8;
        let serial = 1u8;
        let cert_sign_algo = asn1_seq!(ecdsa_with_sha384_oid.clone());
        let issuer = asn1_seq!(asn1_seq!(asn1_seq!(
            common_name_oid.clone(),
            issuer.to_owned()
        )));
        println!("issuer: {:?}", issuer);
        let valid_range = asn1_seq!(issue_ts, expire_ts);
        let subject = asn1_seq!(asn1_seq!(asn1_seq!(
            common_name_oid.clone(),
            subject.to_string(),
        )));
        let pub_key = asn1_seq!(
            asn1_seq!(ec_public_key_oid, prime256v1_oid,),
            BitVec::from_bytes(pub_key),
        );
        let basic_constrain = asn1_seq!(
            basic_constrain_oid,
            true,
            [0x30, 0x03, 0x01, 0x01, 0xff].to_vec()
        );
        let cert_ext = asn1_seq!(basic_constrain,);

        let tbs_cert = asn1_seq!(
            version,
            serial,
            cert_sign_algo,
            issuer,
            valid_range,
            subject,
            pub_key,
            cert_ext,
        );
        IntermediateCert::dump(writer, tbs_cert);
    });

    let sig = key_pair.sign(tbs_cert_der.as_slice()).unwrap();
    let sig_der = yasna::construct_der(|writer| writer.write_der(sig.as_ref()));

    let der = yasna::construct_der(|writer| {
        writer.write_sequence(|writer| {
            writer.next().write_der(tbs_cert_der.as_slice());
            CertSignAlgo::dump(writer.next(), asn1_seq!(ecdsa_with_sha384_oid.clone()));
            writer
                .next()
                .write_bitvec(&BitVec::from_bytes(sig_der.as_slice()));
        });
    });
    Ok(der)
}

fn valid_time_range(cert_valid_seconds: i64) -> Result<(UTCTime, UTCTime)> {
    let issue_ts = get_system_time_seconds()? - 10; // set issued time as 10 seconds earlier
    let mut issue_date_time = DateTime::default();
    seconds_to_datetime(issue_ts, &mut issue_date_time);
    let expire_ts = issue_ts + cert_valid_seconds;
    let mut expire_date_time = DateTime::default();
    seconds_to_datetime(expire_ts, &mut expire_date_time);
    println!("issue_ts: {:?}, date: {}", issue_ts, issue_date_time);
    println!("expire_ts: {:?}, date: {}", expire_ts, expire_date_time);

    let issue_utc_time = match UTCTime::parse(format!("{}", issue_date_time).as_bytes()) {
        Some(t) => t,
        None => {
            bail!("parse issue date time failed")
        }
    };
    let expire_utc_time = match UTCTime::parse(format!("{}", expire_date_time).as_bytes()) {
        Some(t) => t,
        None => {
            bail!("parse expire date time failed")
        }
    };
    Ok((issue_utc_time, expire_utc_time))
}
